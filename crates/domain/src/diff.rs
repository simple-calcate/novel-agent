//! 正文对比：用 `similar` 做行级 Myers/Patience diff，并在改动行内标出字级差异。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use similar::{Algorithm, ChangeTag, TextDiff};

const PREVIEW_CHARS: usize = 48;

/// 某次已落库修订的摘要（不含全文）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionSummary {
    pub revision: u64,
    pub created_at: DateTime<Utc>,
    pub char_count: u32,
    pub preview: String,
    #[serde(default)]
    pub actor: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiffChangeTag {
    Equal,
    Delete,
    Insert,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSpan {
    pub tag: DiffChangeTag,
    pub text: String,
}

/// 一行（或一段）对比结果。`spans` 用于行内高亮；空表示整行同一标签。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub tag: DiffChangeTag,
    pub old_index: Option<u32>,
    pub new_index: Option<u32>,
    pub text: String,
    #[serde(default)]
    pub spans: Vec<DiffSpan>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDiffResult {
    pub summary: String,
    pub inserted_chars: u32,
    pub deleted_chars: u32,
    pub ratio: f32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionDiff {
    pub chapter_id: String,
    pub from_revision: u64,
    pub to_revision: u64,
    #[serde(flatten)]
    pub diff: TextDiffResult,
}

impl RevisionSummary {
    pub fn from_text(
        revision: u64,
        created_at: DateTime<Utc>,
        text: &str,
        actor: impl Into<String>,
    ) -> Self {
        Self {
            revision,
            created_at,
            char_count: text.chars().count() as u32,
            preview: preview_text(text),
            actor: actor.into(),
        }
    }
}

pub fn preview_text(text: &str) -> String {
    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = collapsed.chars();
    let preview: String = chars.by_ref().take(PREVIEW_CHARS).collect();
    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

pub fn summarize_text_diff(old: &str, new: &str) -> String {
    diff_texts(old, new).summary
}

/// 对比两段正文。网文常整段改字，行级 diff + 行内高亮比纯字数摘要有用。
pub fn diff_texts(old: &str, new: &str) -> TextDiffResult {
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .diff_lines(old, new);

    let mut lines = Vec::new();
    let mut inserted_chars = 0u32;
    let mut deleted_chars = 0u32;

    for change in diff.iter_all_inline_changes() {
        let tag = map_tag(change.tag());
        let mut text = String::new();
        let mut spans = Vec::new();
        for (emphasized, value) in change.iter_strings_lossy() {
            let piece = value.into_owned();
            text.push_str(&piece);
            let span_tag = if emphasized {
                tag
            } else {
                DiffChangeTag::Equal
            };
            spans.push(DiffSpan {
                tag: span_tag,
                text: piece,
            });
        }
        match tag {
            DiffChangeTag::Insert => inserted_chars += text.chars().count() as u32,
            DiffChangeTag::Delete => deleted_chars += text.chars().count() as u32,
            DiffChangeTag::Equal => {}
        }
        lines.push(DiffLine {
            tag,
            old_index: change.old_index().map(|index| index as u32),
            new_index: change.new_index().map(|index| index as u32),
            text,
            spans,
        });
    }

    TextDiffResult {
        summary: format_summary(inserted_chars, deleted_chars, old == new),
        inserted_chars,
        deleted_chars,
        ratio: diff.ratio(),
        lines,
    }
}

fn map_tag(tag: ChangeTag) -> DiffChangeTag {
    match tag {
        ChangeTag::Equal => DiffChangeTag::Equal,
        ChangeTag::Delete => DiffChangeTag::Delete,
        ChangeTag::Insert => DiffChangeTag::Insert,
    }
}

fn format_summary(inserted: u32, deleted: u32, identical: bool) -> String {
    if identical {
        return "无改动".into();
    }
    match (inserted, deleted) {
        (0, 0) => "无改动".into(),
        (ins, 0) => format!("+{ins} 字"),
        (0, del) => format!("-{del} 字"),
        (ins, del) => format!("+{ins} 字，-{del} 字"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_texts_have_no_edits() {
        let diff = diff_texts("雾港来客。", "雾港来客。");
        assert_eq!(diff.summary, "无改动");
        assert_eq!(diff.inserted_chars, 0);
        assert_eq!(diff.deleted_chars, 0);
        assert!(diff.lines.iter().all(|line| line.tag == DiffChangeTag::Equal));
    }

    #[test]
    fn chinese_line_change_counts_chars() {
        let old = "雾在潮响前漫进港口。\n林晚站在灯下。";
        let new = "雾在潮响前漫进港口。\n林晚站在码头灯下。";
        let diff = diff_texts(old, new);
        assert!(diff.inserted_chars > 0);
        assert!(diff.deleted_chars > 0);
        assert!(diff
            .lines
            .iter()
            .any(|line| line.tag == DiffChangeTag::Insert || line.tag == DiffChangeTag::Delete));
        assert!(summarize_text_diff(old, new).contains("字"));
    }

    #[test]
    fn preview_truncates_long_text() {
        let text = "雾".repeat(80);
        let preview = preview_text(&text);
        assert!(preview.ends_with('…'));
        assert!(preview.chars().count() < 80);
    }
}
