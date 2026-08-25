//! 块级写作扩展：结构化块编辑（block.save / block.edit）与
//! AI 训练数据一键导出（training.export）。
//!
//! 写作模型模拟 AI reasoning + output：正文块与思考块交替，
//! 思考块可携带标记引用（任务/设定/自定义），导出时按
//! 「思考 → 正文」配对生成训练样本。

use crate::util::{with_repository, with_repository_mut};
use async_trait::async_trait;
use novel_domain::{
    BlockId, BlockKind, BlockSequence, ChapterId, ContentBlock, MarkupRef, ProjectId, Revision,
};
use novel_kernel::{Extension, KernelBuilder, KernelError, Tool, ToolContext};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

fn string_field(input: &Value, key: &str) -> Option<String> {
    input.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn project_id(input: &Value) -> Result<ProjectId, KernelError> {
    string_field(input, "projectId")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| KernelError::ToolFailed {
            tool: "block".into(),
            message: "payload 缺少合法的 projectId".into(),
        })
}

fn chapter_id(input: &Value) -> Result<ChapterId, KernelError> {
    string_field(input, "chapterId")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| KernelError::ToolFailed {
            tool: "block".into(),
            message: "payload 缺少合法的 chapterId".into(),
        })
}

fn base_revision(input: &Value) -> Result<Revision, KernelError> {
    input
        .get("baseRevision")
        .and_then(Value::as_u64)
        .map(Revision)
        .ok_or_else(|| KernelError::ToolFailed {
            tool: "block".into(),
            message: "payload 缺少 baseRevision".into(),
        })
}

/// 把 JSON 数组解析为按 position 稳定排序、重新编号的块列表。
fn parse_blocks(value: &Value) -> Result<Vec<ContentBlock>, KernelError> {
    let items = value.as_array().ok_or_else(|| KernelError::ToolFailed {
        tool: "block".into(),
        message: "blocks 必须是数组".into(),
    })?;
    let mut blocks = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let id: BlockId = item
            .get("id")
            .and_then(Value::as_str)
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| KernelError::ToolFailed {
                tool: "block".into(),
                message: format!("blocks[{index}] 缺少合法的 id"),
            })?;
        let kind = match item.get("kind").and_then(Value::as_str) {
            Some("thinking") => BlockKind::Thinking,
            _ => BlockKind::Body,
        };
        let markup = item
            .get("markup")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|m| serde_json::from_value::<MarkupRef>(m.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        blocks.push(ContentBlock {
            id,
            kind,
            text: item
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            position: item
                .get("position")
                .and_then(Value::as_u64)
                .unwrap_or(index as u64) as u32,
            markup,
        });
    }
    blocks.sort_by_key(|block| block.position);
    for (index, block) in blocks.iter_mut().enumerate() {
        block.position = index as u32;
    }
    Ok(blocks)
}

/// 校验章节归属项目，避免跨项目写入。
fn ensure_chapter_belongs(
    repository: &novel_storage::Repository,
    chapter_id: &ChapterId,
    project_id: &ProjectId,
) -> Result<(), novel_storage::StorageError> {
    let belongs = repository.chapter_project_id(chapter_id)?;
    if belongs.as_ref() != Some(project_id) {
        return Err(novel_domain::DomainError::NotFound(format!(
            "chapter {chapter_id} in project {project_id}"
        ))
        .into());
    }
    Ok(())
}

/// 块级保存：编辑器提交整章块状态（冲突检测基于 baseRevision）。
pub struct BlockSaveTool;

#[async_trait]
impl Tool for BlockSaveTool {
    fn id(&self) -> &str {
        "block.save"
    }

    fn summary(&self) -> &str {
        "提交一章的完整块序列（正文/思考块），带版本冲突检测"
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let project_id = project_id(&input)?;
        let chapter_id = chapter_id(&input)?;
        let revision = base_revision(&input)?;
        let blocks = parse_blocks(&input)?;

        let next = with_repository_mut(ctx.kernel(), |repository| {
            ensure_chapter_belongs(repository, &chapter_id, &project_id)?;
            repository.commit_block_sequence(&chapter_id, revision, &blocks)
        })?;

        Ok(json!({
            "saved": true,
            "revision": next.0,
            "blockCount": blocks.len(),
        }))
    }
}

/// 细粒度块操作（插件/工作流使用）。
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
enum BlockOp {
    Create {
        #[serde(default)]
        block_id: Option<String>,
        #[serde(default)]
        kind: Option<String>,
        #[serde(default)]
        position: Option<u32>,
        #[serde(default)]
        text: String,
        #[serde(default)]
        markup: Vec<MarkupRef>,
    },
    Update {
        block_id: String,
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        kind: Option<String>,
        #[serde(default)]
        markup: Option<Vec<MarkupRef>>,
    },
    Delete {
        block_id: String,
    },
    Move {
        block_id: String,
        position: u32,
    },
    SetMarkup {
        block_id: String,
        markup: Vec<MarkupRef>,
    },
}

fn apply_block_ops(
    blocks: &mut Vec<ContentBlock>,
    ops: &[BlockOp],
) -> Result<(), KernelError> {
    for op in ops {
        match op {
            BlockOp::Create {
                block_id,
                kind,
                position,
                text,
                markup,
            } => {
                let id = block_id
                    .as_ref()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or_else(BlockId::new);
                let kind = match kind.as_deref() {
                    Some("thinking") => BlockKind::Thinking,
                    _ => BlockKind::Body,
                };
                let position = position
                    .unwrap_or(blocks.len() as u32)
                    .min(blocks.len() as u32) as usize;
                blocks.insert(
                    position,
                    ContentBlock {
                        id,
                        kind,
                        text: text.clone(),
                        position: position as u32,
                        markup: markup.clone(),
                    },
                );
            }
            BlockOp::Update {
                block_id,
                text,
                kind,
                markup,
            } => {
                let block = blocks
                    .iter_mut()
                    .find(|block| &block.id.to_string() == block_id)
                    .ok_or_else(|| block_not_found(block_id))?;
                if let Some(text) = text {
                    block.text = text.clone();
                }
                if let Some(kind) = kind {
                    block.kind = if kind == "thinking" {
                        BlockKind::Thinking
                    } else {
                        BlockKind::Body
                    };
                }
                if let Some(markup) = markup {
                    block.markup = markup.clone();
                }
            }
            BlockOp::Delete { block_id } => {
                let index = blocks
                    .iter()
                    .position(|block| &block.id.to_string() == block_id)
                    .ok_or_else(|| block_not_found(block_id))?;
                blocks.remove(index);
            }
            BlockOp::Move { block_id, position } => {
                let index = blocks
                    .iter()
                    .position(|block| &block.id.to_string() == block_id)
                    .ok_or_else(|| block_not_found(block_id))?;
                let block = blocks.remove(index);
                let position = (*position).min(blocks.len() as u32) as usize;
                blocks.insert(position, block);
            }
            BlockOp::SetMarkup { block_id, markup } => {
                let block = blocks
                    .iter_mut()
                    .find(|block| &block.id.to_string() == block_id)
                    .ok_or_else(|| block_not_found(block_id))?;
                block.markup = markup.clone();
            }
        }
    }
    for (index, block) in blocks.iter_mut().enumerate() {
        block.position = index as u32;
    }
    Ok(())
}

fn block_not_found(block_id: &str) -> KernelError {
    KernelError::ToolFailed {
        tool: "block.edit".into(),
        message: format!("block not found: {block_id}"),
    }
}

/// 块级编辑：在最新版本上应用操作，供插件与工作流细粒度修改块。
pub struct BlockEditTool;

#[async_trait]
impl Tool for BlockEditTool {
    fn id(&self) -> &str {
        "block.edit"
    }

    fn summary(&self) -> &str {
        "在章节最新块序列上应用 create/update/delete/move/setMarkup 操作"
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let project_id = project_id(&input)?;
        let chapter_id = chapter_id(&input)?;
        let ops: Vec<BlockOp> = input
            .get("operations")
            .and_then(Value::as_array)
            .ok_or_else(|| KernelError::ToolFailed {
                tool: "block.edit".into(),
                message: "payload 缺少 operations 数组".into(),
            })?
            .iter()
            .map(|item| serde_json::from_value(item.clone()))
            .collect::<Result<_, _>>()
            .map_err(|error| KernelError::ToolFailed {
                tool: "block.edit".into(),
                message: format!("解析操作失败: {error}"),
            })?;

        let (mut blocks, base) = with_repository_mut(ctx.kernel(), |repository| {
            ensure_chapter_belongs(repository, &chapter_id, &project_id)?;
            let base = repository.current_revision(&chapter_id)?;
            let blocks = repository
                .latest_block_sequence(&chapter_id)?
                .map(|sequence| sequence.blocks)
                .unwrap_or_default();
            Ok((blocks, base))
        })?;

        apply_block_ops(&mut blocks, &ops)?;

        let next = with_repository_mut(ctx.kernel(), |repository| {
            repository.commit_block_sequence(&chapter_id, base, &blocks)
        })?;

        Ok(json!({
            "saved": true,
            "revision": next.0,
            "blockCount": blocks.len(),
        }))
    }
}

/// 一个训练样本：思考过程 + 正文输出。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingExample {
    pub thinking: String,
    pub content: String,
}

/// 把块序列按「思考段 → 紧随其后的正文段」配对为训练样本。
/// 相邻同类块合并；正文段无前置思考时 thinking 为空字符串。
pub fn build_training_examples(
    sequence: &BlockSequence,
    include_markup: bool,
) -> Vec<TrainingExample> {
    let mut examples = Vec::new();
    let mut thinking = String::new();
    let mut content = String::new();

    for block in &sequence.blocks {
        match block.kind {
            BlockKind::Thinking => {
                // 新的思考段：先结算上一条思考 + 正文对（若有未结算正文）。
                if !content.is_empty() {
                    examples.push(TrainingExample {
                        thinking: std::mem::take(&mut thinking),
                        content: std::mem::take(&mut content),
                    });
                }
                if !thinking.is_empty() {
                    thinking.push('\n');
                }
                thinking.push_str(&block.text);
                if include_markup {
                    for markup in &block.markup {
                        thinking.push('\n');
                        thinking.push('[');
                        thinking.push_str(&markup.summary());
                        thinking.push(']');
                    }
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
        examples.push(TrainingExample {
            thinking,
            content,
        });
    }

    examples
}

/// 序列化训练样本为指定格式：
/// - jsonl：每行一个 {"thinking": ..., "content": ...}
/// - sharegpt：ShareGPT 对话格式（assistant 消息内嵌 <think>）
/// - r1：纯文本 "<think>...</think>\n\n正文"，样本间以 --- 分隔
pub fn serialize_examples(
    examples: &[TrainingExample],
    format: &str,
) -> Result<String, KernelError> {
    match format {
        "jsonl" => examples
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()
            .map(|lines| lines.join("\n"))
            .map_err(|error| KernelError::ToolFailed {
                tool: "training.export".into(),
                message: format!("序列化 jsonl 失败: {error}"),
            }),
        "sharegpt" => {
            let messages: Vec<Value> = examples
                .iter()
                .map(|example| {
                    json!({
                        "conversations": [
                            { "from": "human", "value": "继续写作" },
                            { "from": "assistant", "value": format!(
                                "<think>\n{}\n</think>\n\n{}",
                                example.thinking, example.content
                            ) },
                        ]
                    })
                })
                .collect();
            serde_json::to_string(&messages).map_err(|error| KernelError::ToolFailed {
                tool: "training.export".into(),
                message: format!("序列化 sharegpt 失败: {error}"),
            })
        }
        "r1" => Ok(examples
            .iter()
            .map(|example| {
                format!(
                    "<think>\n{}\n</think>\n\n{}",
                    example.thinking, example.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")),
        other => Err(KernelError::ToolFailed {
            tool: "training.export".into(),
            message: format!("未知导出格式: {other}（支持 jsonl/sharegpt/r1）"),
        }),
    }
}

/// 一键导出训练数据：把章节块序列转为 AI 训练样本。
pub struct TrainingExportTool;

#[async_trait]
impl Tool for TrainingExportTool {
    fn id(&self) -> &str {
        "training.export"
    }

    fn summary(&self) -> &str {
        "把章节块序列导出为 AI 训练数据（jsonl/sharegpt/r1）"
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let project_id = project_id(&input)?;
        let chapter_id = chapter_id(&input)?;
        let format = string_field(&input, "format").unwrap_or_else(|| "jsonl".into());
        let include_markup = input
            .get("includeMarkup")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let revision = input.get("revision").and_then(Value::as_u64);

        let sequence = with_repository(ctx.kernel(), |repository| {
            ensure_chapter_belongs(repository, &chapter_id, &project_id)?;
            match revision {
                Some(value) => repository.block_sequence(&chapter_id, Revision(value)),
                None => repository.latest_block_sequence(&chapter_id),
            }
        })?;

        let Some(sequence) = sequence else {
            return Ok(json!({
                "format": format,
                "examples": [],
                "output": "",
                "revision": null,
            }));
        };

        let examples = build_training_examples(&sequence, include_markup);
        let output = serialize_examples(&examples, &format)?;

        Ok(json!({
            "format": format,
            "examples": examples,
            "output": output,
            "revision": sequence.revision.0,
        }))
    }
}

/// 块级写作扩展。
pub struct BlocksExtension;

impl Extension for BlocksExtension {
    fn id(&self) -> &str {
        "builtin.blocks"
    }

    fn setup(&self, builder: &mut KernelBuilder) -> Result<(), KernelError> {
        builder.register_tool(BlockSaveTool);
        builder.register_tool(BlockEditTool);
        builder.register_tool(TrainingExportTool);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use novel_domain::BlockId;

    /// 把测试标签确定性映射为合法 UUID，保证 id 定位断言可复现。
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

    fn seq_id(seq: u32) -> String {
        format!("00000000-0000-0000-0000-{seq:012}")
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
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn pairs_thinking_with_following_body() {
        let seq = sequence(vec![
            block("t1", BlockKind::Thinking, "需要铺垫主角动机"),
            block("b1", BlockKind::Body, "林默站在窗前。"),
            block("b2", BlockKind::Body, "他摩挲着旧怀表。"),
            block("t2", BlockKind::Thinking, "引入伏笔：怀表"),
            block("b3", BlockKind::Body, "表盖内侧刻着两个字。"),
        ]);
        let examples = build_training_examples(&seq, false);
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0].thinking, "需要铺垫主角动机");
        assert_eq!(examples[0].content, "林默站在窗前。\n他摩挲着旧怀表。");
        assert_eq!(examples[1].thinking, "引入伏笔：怀表");
        assert_eq!(examples[1].content, "表盖内侧刻着两个字。");
    }

    #[test]
    fn body_without_thinking_gets_empty_thinking() {
        let seq = sequence(vec![
            block("b1", BlockKind::Body, "开篇正文"),
            block("t1", BlockKind::Thinking, "后面才思考"),
            block("b2", BlockKind::Body, "思考后的正文"),
        ]);
        let examples = build_training_examples(&seq, false);
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0].thinking, "");
        assert_eq!(examples[0].content, "开篇正文");
        assert_eq!(examples[1].thinking, "后面才思考");
        assert_eq!(examples[1].content, "思考后的正文");
    }

    #[test]
    fn markup_embedded_into_thinking_when_enabled() {
        let mut thinking = block("t1", BlockKind::Thinking, "角色设定");
        thinking.markup = vec![MarkupRef::Task {
            id: "task-1".into(),
            label: "铺垫动机".into(),
            status: "todo".into(),
        }];
        let seq = sequence(vec![thinking, block("b1", BlockKind::Body, "正文")]);
        let with = build_training_examples(&seq, true);
        let without = build_training_examples(&seq, false);
        assert!(with[0].thinking.contains("[任务[todo]: 铺垫动机]"));
        assert_eq!(without[0].thinking, "角色设定");
    }

    #[test]
    fn ops_create_update_delete_move() {
        let mut blocks = vec![
            block("a", BlockKind::Body, "第一段"),
            block("b", BlockKind::Thinking, "思考"),
        ];
        apply_block_ops(
            &mut blocks,
            &[
                BlockOp::Create {
                    block_id: Some(tag_uuid("c")),
                    kind: Some("body".into()),
                    position: Some(1),
                    text: "插入段".into(),
                    markup: vec![],
                },
                BlockOp::Update {
                    block_id: tag_uuid("a"),
                    text: Some("改后".into()),
                    kind: None,
                    markup: None,
                },
            ],
        )
        .unwrap();
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].text, "改后");
        assert_eq!(blocks[1].text, "插入段");
        assert_eq!(blocks[1].position, 1);

        apply_block_ops(
            &mut blocks,
            &[BlockOp::Move {
                block_id: tag_uuid("c"),
                position: 2,
            }],
        )
        .unwrap();
        assert_eq!(blocks[2].id.to_string(), tag_uuid("c"));

        apply_block_ops(
            &mut blocks,
            &[BlockOp::Delete {
                block_id: tag_uuid("b"),
            }],
        )
        .unwrap();
        assert_eq!(blocks.len(), 2);
        assert!(blocks.iter().all(|block| block.id.to_string() != tag_uuid("b")));
    }

    #[test]
    fn serialize_formats() {
        let examples = vec![TrainingExample {
            thinking: "想一下".into(),
            content: "正文".into(),
        }];
        let jsonl = serialize_examples(&examples, "jsonl").unwrap();
        assert!(jsonl.contains("\"thinking\":\"想一下\""));
        let sharegpt = serialize_examples(&examples, "sharegpt").unwrap();
        assert!(sharegpt.contains("\"from\":\"assistant\""));
        let r1 = serialize_examples(&examples, "r1").unwrap();
        assert!(r1.starts_with("<think>\n想一下\n</think>\n\n正文"));
        assert!(serialize_examples(&examples, "unknown").is_err());
    }
}
