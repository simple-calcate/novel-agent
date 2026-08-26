use crate::provider::{estimate_output_tokens, ModelChunk, ModelRequest};
use crate::KernelError;
use chrono::Utc;
use futures::StreamExt;
use novel_domain::{
    Actor, AgentRunId, ChapterId, ContentPatch, ProjectId, ProposalId, Revision, TextOperation,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentBudget {
    pub max_rounds: u32,
    pub max_tokens: u32,
    pub max_cost_micros: u64,
    pub max_seconds: u64,
}

impl Default for AgentBudget {
    fn default() -> Self {
        Self {
            max_rounds: 2,
            max_tokens: 2048,
            max_cost_micros: 100_000,
            max_seconds: 120,
        }
    }
}

/// 一次 Agent 续写任务的完整输入。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSpec {
    pub id: AgentRunId,
    pub project_id: ProjectId,
    pub chapter_id: ChapterId,
    pub base_revision: Revision,
    pub prompt: String,
    #[serde(default)]
    pub context_text: String,
    #[serde(default)]
    pub budget: AgentBudget,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// 完成后是否在事件总线上发布 `agent.finished` 事件。
    #[serde(default)]
    pub emit_finish_event: bool,
}

fn default_temperature() -> f32 {
    0.8
}

/// 续写执行报告：补丁 + 预算执行情况。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuationReport {
    pub patch: ContentPatch,
    /// 是否因时间或 token 预算耗尽而提前截断。
    pub truncated: bool,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub elapsed_ms: u128,
}

pub(crate) struct StreamOutcome {
    pub text: String,
    pub truncated: bool,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// 消费模型输出流，直到完成、结束或预算（时间/token）耗尽。
/// 预算耗尽时不丢弃已生成内容，而是停止累积并标记 truncated。
pub(crate) async fn consume_stream(
    mut stream: futures::stream::BoxStream<'static, Result<ModelChunk, crate::ModelError>>,
    mut guard: crate::budget::BudgetGuard,
) -> Result<StreamOutcome, KernelError> {
    let mut text = String::new();
    let mut input_tokens = 0u32;
    let mut output_tokens = 0u32;
    let mut truncated = false;

    loop {
        let Some(remaining) = guard.remaining() else {
            truncated = true;
            break;
        };
        match tokio::time::timeout(remaining, stream.next()).await {
            Err(_elapsed) => {
                truncated = true;
                break;
            }
            Ok(None) => break,
            Ok(Some(chunk)) => {
                let chunk = chunk?;
                if let Some(reported) = chunk.input_tokens {
                    input_tokens = input_tokens.max(reported);
                }
                if let Some(reported) = chunk.output_tokens {
                    output_tokens = output_tokens.max(reported);
                }
                if !chunk.text.is_empty() {
                    let estimated = if chunk.output_tokens.is_some() {
                        0
                    } else {
                        estimate_output_tokens(&chunk.text)
                    };
                    let within_budget = guard.record_output(estimated);
                    text.push_str(&chunk.text);
                    if !within_budget {
                        truncated = true;
                        break;
                    }
                }
                if chunk.done {
                    break;
                }
            }
        }
    }

    if output_tokens == 0 {
        output_tokens = estimate_output_tokens(&text);
    }

    Ok(StreamOutcome {
        text,
        truncated,
        input_tokens,
        output_tokens,
    })
}

pub(crate) fn build_request(model: &str, spec: &AgentSpec) -> ModelRequest {
    ModelRequest {
        model: model.to_owned(),
        system_prompt: spec
            .system_prompt
            .clone()
            .unwrap_or_else(|| novel_domain::WRITING_PROTOCOL_SYSTEM.into()),
        user_prompt: format!("{}\n\n任务：{}", spec.context_text, spec.prompt),
        max_tokens: spec.budget.max_tokens,
        temperature: spec.temperature,
    }
}

pub(crate) fn build_patch(spec: &AgentSpec, text: String, model_name: &str) -> ContentPatch {
    ContentPatch {
        id: ProposalId::new(),
        chapter_id: spec.chapter_id.clone(),
        base_revision: spec.base_revision,
        operations: vec![TextOperation::Insert {
            block_id: Default::default(),
            // u32::MAX 在应用时会被夹取到文末，语义为“追加到当前章节末尾”。
            offset: u32::MAX,
            text,
        }],
        rationale: "AI 续写候选".into(),
        created_by: Actor::Agent {
            model: model_name.to_owned(),
        },
        created_at: Utc::now(),
    }
}
