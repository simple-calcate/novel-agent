use chrono::{DateTime, Duration, Utc};
use novel_domain::{Revision, TextOperation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypingSession {
    pub last_input_at: DateTime<Utc>,
    pub last_commit_at: DateTime<Utc>,
    pub composing: bool,
    pub focused: bool,
    pub chars_since_commit: u32,
}

impl TypingSession {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            last_input_at: now,
            last_commit_at: now,
            composing: false,
            focused: true,
            chars_since_commit: 0,
        }
    }

    pub fn should_emit_idle(&self, now: DateTime<Utc>, debounce: Duration, min_chars: u32) -> bool {
        self.focused
            && !self.composing
            && self.chars_since_commit >= min_chars
            && now - self.last_input_at >= debounce
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSummary {
    pub inserted_chars: u32,
    pub deleted_chars: u32,
    pub affected_blocks: Vec<String>,
    pub operation_count: u32,
}

impl ChangeSummary {
    pub fn from_operations(operations: &[TextOperation]) -> Self {
        let mut inserted = 0;
        let mut deleted = 0;
        let mut affected = Vec::new();

        for operation in operations {
            match operation {
                TextOperation::Insert { block_id, text, .. } => {
                    inserted += text.chars().count() as u32;
                    affected.push(block_id.to_string());
                }
                TextOperation::Delete {
                    block_id, length, ..
                } => {
                    deleted += *length;
                    affected.push(block_id.to_string());
                }
                TextOperation::CreateBlock { block_id, text, .. } => {
                    inserted += text.chars().count() as u32;
                    affected.push(block_id.to_string());
                }
            }
        }

        affected.sort();
        affected.dedup();
        Self {
            inserted_chars: inserted,
            deleted_chars: deleted,
            affected_blocks: affected,
            operation_count: operations.len() as u32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuralSignal<T> {
    pub transaction_id: String,
    pub revision_before: Revision,
    pub revision_after: Revision,
    pub source: novel_domain::EventSource,
    pub value: T,
}
