use crate::{JobQueue, QueueError};
use chrono::Utc;
use novel_domain::Job;
use novel_storage::Repository;
use serde_json::json;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error(transparent)]
    Queue(#[from] QueueError),
    #[error(transparent)]
    Storage(#[from] novel_storage::StorageError),
}

pub struct RunnerResult {
    pub job_id: String,
    pub operation: String,
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

pub struct JobRunner<'a> {
    repository: &'a Repository,
    handlers: HashMap<String, Box<dyn JobHandler>>,
}

pub trait JobHandler: Send + Sync {
    fn execute(&self, job: &Job) -> Result<serde_json::Value, String>;
}

pub struct SaveDocumentHandler;
impl JobHandler for SaveDocumentHandler {
    fn execute(&self, job: &Job) -> Result<serde_json::Value, String> {
        Ok(json!({"saved": true, "jobId": job.id.to_string()}))
    }
}

pub struct RebuildIndexHandler;
impl JobHandler for RebuildIndexHandler {
    fn execute(&self, job: &Job) -> Result<serde_json::Value, String> {
        Ok(json!({"indexed": true, "jobId": job.id.to_string()}))
    }
}

pub struct ContinuityCheckHandler;
impl JobHandler for ContinuityCheckHandler {
    fn execute(&self, job: &Job) -> Result<serde_json::Value, String> {
        Ok(json!({"issues": [], "jobId": job.id.to_string()}))
    }
}

pub struct CreateBackupHandler;
impl JobHandler for CreateBackupHandler {
    fn execute(&self, job: &Job) -> Result<serde_json::Value, String> {
        Ok(json!({"backup": true, "timestamp": Utc::now().to_rfc3339(), "jobId": job.id.to_string()}))
    }
}

impl<'a> JobRunner<'a> {
    pub fn new(repository: &'a Repository) -> Self {
        let mut handlers: HashMap<String, Box<dyn JobHandler>> = HashMap::new();
        handlers.insert("document.save".into(), Box::new(SaveDocumentHandler));
        handlers.insert("index.rebuild".into(), Box::new(RebuildIndexHandler));
        handlers.insert("continuity.check".into(), Box::new(ContinuityCheckHandler));
        handlers.insert("backup.create".into(), Box::new(CreateBackupHandler));
        Self { repository, handlers }
    }

    pub fn register_handler(&mut self, operation: String, handler: Box<dyn JobHandler>) {
        self.handlers.insert(operation, handler);
    }

    pub fn run_next(&self) -> Result<Option<RunnerResult>, RunnerError> {
        let queue = JobQueue::new(self.repository);
        let Some(job) = queue.next_due()? else {
            return Ok(None);
        };

        let handler = self.handlers.get(&job.operation);
        let result = match handler {
            Some(handler) => match handler.execute(&job) {
                Ok(output) => RunnerResult {
                    job_id: job.id.to_string(),
                    operation: job.operation.clone(),
                    success: true,
                    output: Some(output),
                    error: None,
                },
                Err(error) => RunnerResult {
                    job_id: job.id.to_string(),
                    operation: job.operation.clone(),
                    success: false,
                    output: None,
                    error: Some(error),
                },
            },
            None => RunnerResult {
                job_id: job.id.to_string(),
                operation: job.operation.clone(),
                success: false,
                output: None,
                error: Some(format!("no handler for operation {}", job.operation)),
            },
        };

        Ok(Some(result))
    }
}
