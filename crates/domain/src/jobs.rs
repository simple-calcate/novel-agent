use crate::{JobId, ProjectId, WorkflowId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: JobId,
    pub project_id: ProjectId,
    pub workflow_id: Option<WorkflowId>,
    pub operation: String,
    pub payload: Value,
    pub priority: i32,
    pub status: JobStatus,
    pub idempotency_key: String,
    pub depends_on: Vec<JobId>,
    pub attempts: u32,
    pub max_attempts: u32,
    pub run_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub causation_id: Option<String>,
    pub causation_depth: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobStatus {
    Pending,
    Blocked,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    DeadLetter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRule {
    pub id: WorkflowId,
    pub project_id: ProjectId,
    pub name: String,
    pub enabled: bool,
    pub trigger: WorkflowTrigger,
    pub conditions: Vec<WorkflowCondition>,
    pub actions: Vec<WorkflowAction>,
    pub priority: i32,
    pub cooldown_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowTrigger {
    pub event_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowCondition {
    pub path: String,
    pub operator: ConditionOperator,
    pub value: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConditionOperator {
    Eq,
    NotEq,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    Exists,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WorkflowAction {
    SaveDocument,
    RebuildIndex,
    CheckContinuity,
    GenerateContinuation {
        max_tokens: u32,
    },
    CreateBackup,
    RunAgent {
        prompt: String,
    },
    RunPluginOperation {
        plugin_id: String,
        operation: String,
        input: Value,
    },
}
