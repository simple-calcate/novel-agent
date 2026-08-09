use chrono::Utc;
use novel_domain::{Job, JobId, JobStatus, ProjectId, WorkflowAction, WorkflowId};
use novel_storage::Repository;
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

const MAX_CAUSATION_DEPTH: u32 = 8;

#[derive(Debug, Error)]
pub enum QueueError {
    #[error(transparent)]
    Storage(#[from] novel_storage::StorageError),
    #[error("causation depth exceeded")]
    CausationDepthExceeded,
}

pub struct JobQueue<'a> {
    repository: &'a Repository,
}

impl<'a> JobQueue<'a> {
    pub fn new(repository: &'a Repository) -> Self {
        Self { repository }
    }

    pub fn enqueue(
        &self,
        project_id: ProjectId,
        workflow_id: Option<WorkflowId>,
        action: &WorkflowAction,
        payload: Value,
        priority: i32,
        idempotency_key: String,
        causation_id: Option<String>,
        causation_depth: u32,
    ) -> Result<Option<JobId>, QueueError> {
        if causation_depth > MAX_CAUSATION_DEPTH {
            return Err(QueueError::CausationDepthExceeded);
        }

        let operation = operation_name(action).to_owned();
        let now = Utc::now();
        let job = Job {
            id: JobId::new(),
            project_id,
            workflow_id,
            operation,
            payload,
            priority,
            status: JobStatus::Pending,
            idempotency_key,
            depends_on: Vec::new(),
            attempts: 0,
            max_attempts: 3,
            run_at: now,
            deadline: None,
            causation_id,
            causation_depth,
            created_at: now,
            updated_at: now,
        };

        let inserted = self.repository.enqueue_job(&job)?;
        Ok(inserted.then_some(job.id))
    }

    pub fn next_due(&self) -> Result<Option<Job>, QueueError> {
        Ok(self.repository.next_runnable_job(&Utc::now().to_rfc3339())?)
    }
}

fn operation_name(action: &WorkflowAction) -> &'static str {
    match action {
        WorkflowAction::SaveDocument => "document.save",
        WorkflowAction::RebuildIndex => "index.rebuild",
        WorkflowAction::CheckContinuity => "continuity.check",
        WorkflowAction::GenerateContinuation { .. } => "agent.continuation",
        WorkflowAction::CreateBackup => "backup.create",
        WorkflowAction::RunAgent { .. } => "agent.run",
        WorkflowAction::RunPluginOperation { .. } => "plugin.operation",
    }
}

pub fn stable_idempotency_key(event_id: &str, workflow_id: &Uuid, action: &str) -> String {
    format!("{event_id}:{workflow_id}:{action}")
}
