//! 墨枢写作协议 v2：把块序列变成可训练的拍级样本。
//!
//! 每条样本的上文默认从**本章开头**排到本拍之前，思考和正文都保留。
//! 规范文本：`docs/writing-protocol.md`。

use crate::{BlockKind, BlockSequence, ContentBlock};
use serde::{Deserialize, Serialize};

/// 协议版本。导出文件带上，避免旧转换脚本误读新字段。
pub const WRITING_PROTOCOL_VERSION: u32 = 2;

/// 写入 ShareGPT / Alpaca 的稳定系统短指令。改措辞必须同步协议文档。
pub const WRITING_PROTOCOL_SYSTEM: &str =
    "你按墨枢写作协议写网文：先思考本拍决策，再写读者可见正文。思考不是人物内心独白，正文不含作者备注。思考里的 @人物/@伏笔 是写作标签，不必当成数据库实体。";

const THINKING_PREFIX: &str = "【思考】";

const GOLD_THINKING_MIN: usize = 16;
const GOLD_BODY_MIN: usize = 40;
const USABLE_THINKING_MIN: usize = 8;
const USABLE_BODY_MIN: usize = 24;
const COPY_DETECT_MIN: usize = 16;

/// 训练样本质量。默认导出 gold + usable。
/// 判别顺序依赖变体声明顺序：Skip < Usable < Gold。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExampleQuality {
    Skip,
    Usable,
    Gold,
}

impl ExampleQuality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Skip => "skip",
            Self::Usable => "usable",
            Self::Gold => "gold",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "skip" => Some(Self::Skip),
            "usable" => Some(Self::Usable),
            "gold" => Some(Self::Gold),
            _ => None,
        }
    }
}

/// 思考槽位。标签在导出时从文本解析，不单独落库。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingSlots {
    pub intent: Option<String>,
    pub constraints: Option<String>,
    pub technique: Option<String>,
    pub must_show: Option<String>,
    pub must_not: Option<String>,
    /// 未识别标签的剩余思考。
    pub notes: String,
}

impl ThinkingSlots {
    pub fn has_intent(&self) -> bool {
        self.intent
            .as_ref()
            .map(|value| !value.is_empty())
            .unwrap_or(false)
    }
}

/// 一条拍级训练样本。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingExample {
    pub protocol_version: u32,
    pub beat_index: u32,
    pub quality: ExampleQuality,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skip_reasons: Vec<String>,
    /// 本拍之前、从章首排下来的全部内容（思考 + 正文）。开篇可为空。
    pub context: String,
    /// 短任务句，不含本拍思考。
    pub instruction: String,
    pub thinking: String,
    pub content: String,
    pub slots: ThinkingSlots,
}

impl TrainingExample {
    pub fn assistant_r1(&self) -> String {
        let thinking = self.thinking.trim();
        if thinking.is_empty() {
            self.content.clone()
        } else {
            format!("<think>\n{}\n</think>\n\n{}", thinking, self.content)
        }
    }

    pub fn human_prompt(&self) -> String {
        if self.context.is_empty() {
            self.instruction.clone()
        } else {
            format!("{}\n\n{}", self.context, self.instruction)
        }
    }
}

/// 把块序列按拍切开并打分。`chapter_title` 只用于开篇 instruction。
pub fn build_training_examples(
    sequence: &BlockSequence,
    include_markup: bool,
    chapter_title: Option<&str>,
) -> Vec<TrainingExample> {
    build_training_examples_from_blocks(&sequence.blocks, include_markup, chapter_title)
}

pub fn build_training_examples_from_blocks(
    blocks: &[ContentBlock],
    include_markup: bool,
    chapter_title: Option<&str>,
) -> Vec<TrainingExample> {
    let mut examples = Vec::new();
    let mut thinking = String::new();
    let mut content = String::new();
    let mut preceding = String::new();

    for block in blocks {
        match block.kind {
            BlockKind::Thinking => {
                if !content.is_empty() {
                    examples.push(finish_example(
                        &thinking,
                        &content,
                        &preceding,
                        chapter_title,
                        examples.len() as u32,
                    ));
                    append_beat_to_transcript(&mut preceding, &thinking, &content);
                    thinking.clear();
                    content.clear();
                }
                if !thinking.is_empty() {
                    thinking.push('\n');
                }
                thinking.push_str(&block.text);
                if include_markup {
                    append_markup_if_missing(&mut thinking, &block.markup);
                }
            }
            BlockKind::Body => {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(&block.text);
            }
        }
    }

    if !content.is_empty() {
        examples.push(finish_example(
            &thinking,
            &content,
            &preceding,
            chapter_title,
            examples.len() as u32,
        ));
    }

    examples
}

fn finish_example(
    thinking: &str,
    content: &str,
    preceding: &str,
    chapter_title: Option<&str>,
    beat_index: u32,
) -> TrainingExample {
    let thinking = thinking.trim().to_owned();
    let content = content.trim().to_owned();
    let slots = parse_thinking_slots(&thinking);
    let (quality, skip_reasons) = grade_example(&thinking, &content, &slots);
    let context = preceding.trim().to_owned();
    let instruction = beat_instruction(&context, chapter_title);
    TrainingExample {
        protocol_version: WRITING_PROTOCOL_VERSION,
        beat_index,
        quality,
        skip_reasons,
        context,
        instruction,
        thinking,
        content,
        slots,
    }
}

fn fold_colons(text: &str) -> String {
    text.replace('：', ":")
}

fn append_markup_if_missing(thinking: &mut String, markup: &[crate::MarkupRef]) {
    for item in markup {
        let summary = item.summary();
        if fold_colons(thinking).contains(&fold_colons(&summary)) {
            continue;
        }
        if !thinking.is_empty() {
            thinking.push('\n');
        }
        thinking.push('[');
        thinking.push_str(&summary);
        thinking.push(']');
    }
}

fn append_beat_to_transcript(out: &mut String, thinking: &str, content: &str) {
    let thinking = thinking.trim();
    let content = content.trim();
    if !thinking.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(THINKING_PREFIX);
        out.push_str(thinking);
    }
    if !content.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(content);
    }
}

fn beat_instruction(context: &str, chapter_title: Option<&str>) -> String {
    if context.is_empty() {
        match chapter_title
            .map(str::trim)
            .filter(|title| !title.is_empty())
        {
            Some(title) => format!("写下《{title}》的开篇。"),
            None => "写下本章开篇。".into(),
        }
    } else {
        "续写下一段。".into()
    }
}

/// 解析思考槽位。同行 `标签：内容`；连续无标签行并入当前槽或 notes。
pub fn parse_thinking_slots(thinking: &str) -> ThinkingSlots {
    let mut slots = ThinkingSlots::default();
    let mut current: Option<SlotKind> = None;
    let mut notes: Vec<String> = Vec::new();

    for raw in thinking.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((kind, rest)) = split_slot_line(line) {
            current = Some(kind);
            append_slot(&mut slots, kind, rest);
        } else if let Some(kind) = current {
            append_slot(&mut slots, kind, line);
        } else {
            notes.push(line.to_owned());
        }
    }

    slots.notes = notes.join("\n");
    slots
}

#[derive(Clone, Copy)]
enum SlotKind {
    Intent,
    Constraints,
    Technique,
    MustShow,
    MustNot,
}

fn split_slot_line(line: &str) -> Option<(SlotKind, &str)> {
    let (label, rest) = line.split_once('：').or_else(|| line.split_once(':'))?;
    let kind = match normalize_label(label).as_str() {
        "意图" | "目标" | "目的" => SlotKind::Intent,
        "约束" | "限制" | "正史" => SlotKind::Constraints,
        "手法" | "写法" | "节奏" => SlotKind::Technique,
        "兑现" | "必须" | "出场" => SlotKind::MustShow,
        "禁止" | "勿" | "不能" => SlotKind::MustNot,
        _ => return None,
    };
    Some((kind, rest.trim()))
}

fn normalize_label(label: &str) -> String {
    label
        .trim()
        .trim_start_matches(['[', '【', ' ', '\t'])
        .trim_end_matches([']', '】', ' ', '\t'])
        .to_owned()
}

fn append_slot(slots: &mut ThinkingSlots, kind: SlotKind, text: &str) {
    if text.is_empty() {
        return;
    }
    let target = match kind {
        SlotKind::Intent => &mut slots.intent,
        SlotKind::Constraints => &mut slots.constraints,
        SlotKind::Technique => &mut slots.technique,
        SlotKind::MustShow => &mut slots.must_show,
        SlotKind::MustNot => &mut slots.must_not,
    };
    match target {
        Some(existing) => {
            existing.push('\n');
            existing.push_str(text);
        }
        None => *target = Some(text.to_owned()),
    }
}

pub fn grade_example(
    thinking: &str,
    content: &str,
    slots: &ThinkingSlots,
) -> (ExampleQuality, Vec<String>) {
    let thinking_len = thinking.chars().count();
    let content_len = content.chars().count();
    let mut reasons = Vec::new();

    if thinking.is_empty() {
        reasons.push("emptyThinking".into());
    } else if thinking_len < USABLE_THINKING_MIN {
        reasons.push("thinkingTooShort".into());
    }
    if content_len < USABLE_BODY_MIN {
        reasons.push("bodyTooShort".into());
    }
    if is_copied_across_layers(thinking, content) {
        reasons.push("thinkingCopiedToBody".into());
    }
    if body_looks_like_notes(content) {
        reasons.push("bodyLooksLikeNotes".into());
    }

    if !reasons.is_empty() {
        return (ExampleQuality::Skip, reasons);
    }

    let gold =
        slots.has_intent() && thinking_len >= GOLD_THINKING_MIN && content_len >= GOLD_BODY_MIN;
    if gold {
        (ExampleQuality::Gold, Vec::new())
    } else {
        (ExampleQuality::Usable, Vec::new())
    }
}

fn is_copied_across_layers(thinking: &str, content: &str) -> bool {
    let thinking = thinking.trim();
    let content = content.trim();
    if thinking.chars().count() < COPY_DETECT_MIN && content.chars().count() < COPY_DETECT_MIN {
        return false;
    }
    if thinking.chars().count() >= COPY_DETECT_MIN && content.contains(thinking) {
        return true;
    }
    if content.chars().count() >= COPY_DETECT_MIN && thinking.contains(content) {
        return true;
    }
    false
}

fn body_looks_like_notes(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    if lower.contains("todo") || lower.contains("fixme") {
        return true;
    }
    if content.contains("（作者") || content.contains("(作者") || content.contains("【作者")
    {
        return true;
    }
    if content.contains("此处待") {
        return true;
    }
    content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with(">> ")
            || trimmed.starts_with("意图：")
            || trimmed.starts_with("意图:")
            || trimmed.starts_with("约束：")
            || trimmed.starts_with("约束:")
            || trimmed.starts_with("伏笔：")
            || trimmed.starts_with("伏笔:")
    })
}

pub fn filter_examples(
    examples: &[TrainingExample],
    min_quality: ExampleQuality,
) -> Vec<TrainingExample> {
    examples
        .iter()
        .filter(|example| example.quality >= min_quality)
        .cloned()
        .collect()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityCounts {
    pub gold: usize,
    pub usable: usize,
    pub skip: usize,
}

pub fn quality_counts(examples: &[TrainingExample]) -> QualityCounts {
    let mut counts = QualityCounts::default();
    for example in examples {
        match example.quality {
            ExampleQuality::Gold => counts.gold += 1,
            ExampleQuality::Usable => counts.usable += 1,
            ExampleQuality::Skip => counts.skip += 1,
        }
    }
    counts
}

/// 序列化训练样本。
///
/// - jsonl：原生字段
/// - sharegpt：system + human + assistant
/// - alpaca：instruction / input / output
/// - r1：`<think>` 文本，样本间 `---`
pub fn serialize_examples(examples: &[TrainingExample], format: &str) -> Result<String, String> {
    match format {
        "jsonl" => examples
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .map(|lines| lines.join("\n"))
            .map_err(|error| format!("序列化 jsonl 失败: {error}")),
        "sharegpt" => {
            let messages: Vec<serde_json::Value> = examples
                .iter()
                .map(|example| {
                    serde_json::json!({
                        "conversations": [
                            { "from": "system", "value": WRITING_PROTOCOL_SYSTEM },
                            { "from": "human", "value": example.human_prompt() },
                            { "from": "assistant", "value": example.assistant_r1() },
                        ]
                    })
                })
                .collect();
            serde_json::to_string(&messages)
                .map_err(|error| format!("序列化 sharegpt 失败: {error}"))
        }
        "alpaca" => {
            let rows: Vec<serde_json::Value> = examples
                .iter()
                .map(|example| {
                    serde_json::json!({
                        "instruction": WRITING_PROTOCOL_SYSTEM,
                        "input": example.human_prompt(),
                        "output": example.assistant_r1(),
                    })
                })
                .collect();
            serde_json::to_string(&rows).map_err(|error| format!("序列化 alpaca 失败: {error}"))
        }
        "r1" => Ok(examples
            .iter()
            .map(TrainingExample::assistant_r1)
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")),
        other => Err(format!(
            "未知导出格式: {other}（支持 jsonl/sharegpt/alpaca/r1）"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlockId, ChapterId, ContentBlock, MarkupRef, Revision};
    use chrono::Utc;

    fn tag_uuid(tag: &str) -> String {
        let mut bytes = [0u8; 16];
        let n = tag.len().min(16);
        bytes[..n].copy_from_slice(&tag.as_bytes()[..n]);
        uuid::Uuid::from_bytes(bytes).to_string()
    }

    fn block(id: &str, kind: BlockKind, text: &str) -> ContentBlock {
        ContentBlock {
            id: BlockId(uuid::Uuid::parse_str(&tag_uuid(id)).unwrap()),
            kind,
            text: text.to_owned(),
            position: 0,
            markup: vec![],
        }
    }

    fn sequence(blocks: Vec<ContentBlock>) -> BlockSequence {
        let mut blocks = blocks;
        for (index, block) in blocks.iter_mut().enumerate() {
            block.position = index as u32;
        }
        BlockSequence {
            chapter_id: ChapterId::new(),
            revision: Revision(1),
            blocks,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn pairs_thinking_with_following_body() {
        let seq = sequence(vec![
            block(
                "t1",
                BlockKind::Thinking,
                "意图：让读者感到怀表有秘密，但不揭晓是谁留的",
            ),
            block(
                "b1",
                BlockKind::Body,
                "林默站在窗前。雾已经漫过码头的铁索，潮声一下一下敲着船帮。",
            ),
            block(
                "b2",
                BlockKind::Body,
                "他摩挲着那只旧怀表，盖沿的铜锈蹭在指腹上，像有人刚刚摸过。",
            ),
            block(
                "t2",
                BlockKind::Thinking,
                "意图：让表盖内侧的两个字入镜，但不解释含义",
            ),
            block(
                "b3",
                BlockKind::Body,
                "他把表盖掀开一条缝。内侧刻着两个字，笔画浅得像被潮气咬过，灯火一晃，那两个字又隐回去了。",
            ),
        ]);
        let examples = build_training_examples(&seq, false, Some("雾港来客"));
        assert_eq!(examples.len(), 2);
        assert!(examples[0].thinking.contains("意图：让读者感到怀表有秘密"));
        assert!(examples[0].content.contains("林默站在窗前。"));
        assert_eq!(examples[0].instruction, "写下《雾港来客》的开篇。");
        assert_eq!(examples[0].context, "");
        assert_eq!(examples[1].instruction, "续写下一段。");
        assert!(examples[1].context.contains("【思考】意图：让读者感到怀表有秘密"));
        assert!(examples[1].context.contains("林默站在窗前。"));
        assert!(!examples[1].context.contains("表盖掀开"));
        assert_eq!(examples[0].quality, ExampleQuality::Gold);
        assert_eq!(examples[1].quality, ExampleQuality::Gold);
    }

    #[test]
    fn body_without_thinking_is_skip() {
        let seq = sequence(vec![
            block(
                "b1",
                BlockKind::Body,
                "开篇没有思考，这段正文再长也不能进默认训练集，只会被标成 skip。",
            ),
            block(
                "t1",
                BlockKind::Thinking,
                "意图：后面才补思考，让这一拍成为金标样本",
            ),
            block(
                "b2",
                BlockKind::Body,
                "思考后的正文必须写够一个段落，潮声、雾和人影都要落到纸上，连灯笼的光都到不了这边。",
            ),
        ]);
        let examples = build_training_examples(&seq, false, None);
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0].quality, ExampleQuality::Skip);
        assert!(examples[0].skip_reasons.contains(&"emptyThinking".into()));
        assert_eq!(examples[1].quality, ExampleQuality::Gold);
        let kept = filter_examples(&examples, ExampleQuality::Usable);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].beat_index, 1);
    }

    #[test]
    fn parses_slot_synonyms_and_notes() {
        let slots = parse_thinking_slots(
            "先定调\n目标：让雾先压过来\n限制：不解释来源\n写法：短句\n必须：潮声\n勿：开场独白",
        );
        assert_eq!(slots.intent.as_deref(), Some("让雾先压过来"));
        assert_eq!(slots.constraints.as_deref(), Some("不解释来源"));
        assert_eq!(slots.technique.as_deref(), Some("短句"));
        assert_eq!(slots.must_show.as_deref(), Some("潮声"));
        assert_eq!(slots.must_not.as_deref(), Some("开场独白"));
        assert_eq!(slots.notes, "先定调");
    }

    #[test]
    fn skips_copied_thinking_and_author_notes() {
        let copied = "林默站在窗前看雾渗进港口的石缝里。";
        let seq = sequence(vec![
            block("t1", BlockKind::Thinking, copied),
            block("b1", BlockKind::Body, copied),
        ]);
        let examples = build_training_examples(&seq, false, None);
        assert_eq!(examples[0].quality, ExampleQuality::Skip);
        assert!(examples[0]
            .skip_reasons
            .contains(&"thinkingCopiedToBody".into()));

        let notes = sequence(vec![
            block("t2", BlockKind::Thinking, "意图：补一段说明"),
            block(
                "b2",
                BlockKind::Body,
                "（作者注：这里要加打戏）林默拔剑。这句要写得很长才过正文下限。",
            ),
        ]);
        let examples = build_training_examples(&notes, false, None);
        assert_eq!(examples[0].quality, ExampleQuality::Skip);
        assert!(examples[0]
            .skip_reasons
            .contains(&"bodyLooksLikeNotes".into()));
    }

    #[test]
    fn markup_embedded_when_enabled() {
        let mut thinking = block("t1", BlockKind::Thinking, "意图：角色出场");
        thinking.markup = vec![MarkupRef::Task {
            id: "task-1".into(),
            label: "铺垫动机".into(),
            status: "todo".into(),
        }];
        let seq = sequence(vec![
            thinking,
            block("b1", BlockKind::Body, "林默站在窗前，潮声先于人影到来。"),
        ]);
        let with = build_training_examples(&seq, true, None);
        let without = build_training_examples(&seq, false, None);
        assert!(with[0].thinking.contains("[任务[todo]: 铺垫动机]"));
        assert_eq!(without[0].thinking, "意图：角色出场");
    }

    #[test]
    fn story_tag_is_author_label_not_canon_row() {
        let tag = MarkupRef::Tag {
            id: String::new(),
            kind: "人物".into(),
            label: "林默".into(),
            note: String::new(),
        };
        assert_eq!(tag.summary(), "@人物：林默");
        let mut thinking = block("t1", BlockKind::Thinking, "意图：让林默出场\n@人物:林默");
        thinking.markup = vec![tag.clone()];
        let seq = sequence(vec![
            thinking,
            block(
                "b1",
                BlockKind::Body,
                "林默站在窗前。雾已经漫过码头的铁索，潮声一下一下敲着船帮。",
            ),
        ]);
        let examples = build_training_examples(&seq, true, None);
        assert!(examples[0].thinking.contains("@人物:林默"));
        assert!(
            !examples[0].thinking.contains("[@人物：林默]"),
            "思考里已经写了标签，不要再贴一份摘要"
        );

        let mut untitled = block("t2", BlockKind::Thinking, "意图：让林默出场");
        untitled.markup = vec![tag];
        let seq = sequence(vec![
            untitled,
            block(
                "b2",
                BlockKind::Body,
                "林默站在窗前。雾已经漫过码头的铁索，潮声一下一下敲着船帮。",
            ),
        ]);
        let examples = build_training_examples(&seq, true, None);
        assert!(examples[0].thinking.contains("[@人物：林默]"));
    }

    #[test]
    fn context_from_chapter_start_keeps_all_thinking_and_body() {
        let long_body = format!("{}。", "雾已经漫过码头的铁索".repeat(40));
        let seq = sequence(vec![
            block("t1", BlockKind::Thinking, "意图：铺一整段冷开场，让雾先压住港口"),
            block("b1", BlockKind::Body, &long_body),
            block("t2", BlockKind::Thinking, "意图：再写人影，但不让读者看清脸"),
            block(
                "b2",
                BlockKind::Body,
                "远处有人把灯笼从帆布里掏出来，光却到不了这边。石阶湿了一圈。",
            ),
        ]);
        let examples = build_training_examples(&seq, false, Some("雾港来客"));
        assert_eq!(examples.len(), 2);
        assert!(examples[1].context.starts_with("【思考】意图：铺一整段冷开场"));
        assert!(examples[1].context.contains(&long_body));
        assert!(!examples[1].context.contains("灯笼从帆布"));
    }

    #[test]
    fn serialize_never_uses_dummy_continue_prompt() {
        let example = TrainingExample {
            protocol_version: 2,
            beat_index: 0,
            quality: ExampleQuality::Gold,
            skip_reasons: vec![],
            context: "林默站在窗前。".into(),
            instruction: "续写下一段。".into(),
            thinking: "意图：冷开场".into(),
            content: "雾先于脚步声漫进港口。".into(),
            slots: ThinkingSlots {
                intent: Some("冷开场".into()),
                ..ThinkingSlots::default()
            },
        };
        let jsonl = serialize_examples(&[example.clone()], "jsonl").unwrap();
        assert!(jsonl.contains("\"thinking\":\"意图：冷开场\""));
        assert!(jsonl.contains("\"instruction\":\"续写下一段。\""));
        let sharegpt = serialize_examples(&[example.clone()], "sharegpt").unwrap();
        assert!(sharegpt.contains(WRITING_PROTOCOL_SYSTEM));
        assert!(sharegpt.contains("林默站在窗前。"));
        assert!(!sharegpt.contains("继续写作"));
        let alpaca = serialize_examples(&[example.clone()], "alpaca").unwrap();
        assert!(alpaca.contains("\"input\""));
        let r1 = serialize_examples(&[example], "r1").unwrap();
        assert!(r1.starts_with("<think>\n意图：冷开场\n</think>\n\n雾先于脚步声漫进港口。"));
        assert!(serialize_examples(&[], "unknown").is_err());
    }
}
