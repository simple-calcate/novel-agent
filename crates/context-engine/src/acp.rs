use chrono::Utc;
use novel_domain::{ContextBlockId, SessionId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreMessage {
    pub id: String,
    pub role: MessageRole,
    pub text: String,
    pub tokens: u32,
    pub protected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionBlock {
    pub id: ContextBlockId,
    pub session_id: SessionId,
    pub tier: u8,
    pub topic: Option<String>,
    pub summary: String,
    pub direct_message_ids: Vec<String>,
    pub active: bool,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextState {
    pub session_id: SessionId,
    pub blocks: Vec<CompressionBlock>,
    pub next_ref: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NudgeDecision {
    pub should_inject: bool,
    pub reason: String,
    pub usage_ratio: f32,
}

pub struct ContextEngine {
    pub context_limit: u32,
    pub min_context_limit_pct: f32,
    pub preserve_recent_messages: usize,
}

impl ContextEngine {
    pub fn process_turn(
        &self,
        messages: Vec<CoreMessage>,
        state: ContextState,
    ) -> (Vec<CoreMessage>, ContextState, NudgeDecision) {
        let token_count: u32 = messages.iter().map(|message| message.tokens).sum();
        let usage = token_count as f32 / self.context_limit.max(1) as f32;
        let nudge = NudgeDecision {
            should_inject: usage >= self.min_context_limit_pct,
            reason: if usage >= self.min_context_limit_pct {
                "上下文超过压缩阈值".into()
            } else {
                "上下文仍在预算内".into()
            },
            usage_ratio: usage,
        };

        for block in state.blocks.iter().filter(|block| block.active) {
            let Some(first) = block.direct_message_ids.first() else {
                // 空块（无覆盖消息）直接跳过，避免越界
                continue;
            };
            let last = &block.direct_message_ids[block.direct_message_ids.len() - 1];
            let start = messages.iter().position(|message| &message.id == first);
            let end = messages.iter().position(|message| &message.id == last);
            if let (Some(start), Some(end)) = (start, end) {
                let summary = CoreMessage {
                    id: format!("block:{}", block.id),
                    role: MessageRole::System,
                    text: block.summary.clone(),
                    tokens: estimate_tokens(&block.summary),
                    protected: true,
                };
                let mut rendered = Vec::with_capacity(messages.len() + 1);
                rendered.extend_from_slice(&messages[..start]);
                rendered.push(summary);
                rendered.extend_from_slice(&messages[end + 1..]);
                return (rendered, state, nudge);
            }
        }

        (messages, state, nudge)
    }

    pub fn apply_compression(
        &self,
        messages: &[CoreMessage],
        mut state: ContextState,
        summary: String,
        topic: Option<String>,
    ) -> ContextState {
        let preserve = self.preserve_recent_messages.min(messages.len());
        let end = messages.len().saturating_sub(preserve);
        let ids: Vec<String> = messages[..end]
            .iter()
            .filter(|message| !message.protected)
            .map(|message| message.id.clone())
            .collect();
        if ids.is_empty() {
            return state;
        }

        state.blocks.push(CompressionBlock {
            id: ContextBlockId::new(),
            session_id: state.session_id.clone(),
            tier: 1,
            topic,
            summary,
            direct_message_ids: ids,
            active: true,
            created_at: Utc::now(),
        });
        state
    }

    pub fn search<'a>(&self, state: &'a ContextState, query: &str) -> Vec<&'a CompressionBlock> {
        state
            .blocks
            .iter()
            .filter(|block| {
                block.summary.contains(query)
                    || block
                        .topic
                        .as_ref()
                        .is_some_and(|topic| topic.contains(query))
            })
            .collect()
    }
}

pub fn estimate_tokens(text: &str) -> u32 {
    text.chars().count().div_ceil(2) as u32
}
