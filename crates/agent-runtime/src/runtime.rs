use crate::{ModelProvider, ModelRequest};
use chrono::Utc;
use futures::StreamExt;
use novel_domain::{
    Actor, AgentRunId, ChapterId, ContentPatch, ProposalId, Revision, TextOperation,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentBudget {
    pub max_rounds: u32,
    pub max_tokens: u32,
    pub max_cost_micros: u64,
    pub max_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRun {
    pub id: AgentRunId,
    pub chapter_id: ChapterId,
    pub base_revision: Revision,
    pub prompt: String,
    pub budget: AgentBudget,
    pub started_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum AgentRuntimeError {
    #[error(transparent)]
    Provider(#[from] crate::ModelError),
}

pub struct AgentRuntime {
    provider: Box<dyn ModelProvider>,
    model_name: String,
}

impl AgentRuntime {
    pub fn new(provider: impl ModelProvider + 'static) -> Self {
        Self {
            provider: Box::new(provider),
            model_name: "default".into(),
        }
    }

    pub fn new_provider(provider: Box<dyn ModelProvider>) -> Self {
        Self {
            provider,
            model_name: "default".into(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model_name = model.into();
        self
    }

    pub async fn run_continuation(
        &self,
        run: AgentRun,
        context_text: String,
    ) -> Result<ContentPatch, AgentRuntimeError> {
        let request = ModelRequest {
            model: self.model_name.clone(),
            system_prompt: "你是网文续写助手，必须遵守给定设定。".into(),
            user_prompt: format!("{}\n\n任务：{}", context_text, run.prompt),
            max_tokens: run.budget.max_tokens,
            temperature: 0.8,
        };

        let mut stream = self.provider.stream(request).await?;
        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            text.push_str(&chunk?.text);
        }

        Ok(ContentPatch {
            id: ProposalId::new(),
            chapter_id: run.chapter_id,
            base_revision: run.base_revision,
            operations: vec![TextOperation::Insert {
                block_id: Default::default(),
                offset: u32::MAX,
                text,
            }],
            rationale: "AI 续写候选".into(),
            created_by: Actor::Agent {
                model: self.provider.name().into(),
            },
            created_at: Utc::now(),
        })
    }
}
