use super::Repository;
use crate::StorageError;
use chrono::Utc;
use novel_domain::{DomainError, Job, JobId, JobStatus, ProjectId};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

const JOB_COLUMNS: &str = "id, project_id, workflow_id, operation, payload_json, priority, status,
                    idempotency_key, depends_on_json, attempts, max_attempts, run_at,
                    deadline, causation_id, causation_depth, created_at, updated_at";

impl Repository {
    pub fn enqueue_job(&self, job: &Job) -> Result<bool, StorageError> {
        self.transact(|tx| {
            let inserted = tx.execute(
                "INSERT OR IGNORE INTO jobs(
                    id, project_id, workflow_id, operation, payload_json, priority, status,
                    idempotency_key, depends_on_json, attempts, max_attempts, run_at,
                    deadline, causation_id, causation_depth, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    job.id.to_string(),
                    job.project_id.to_string(),
                    job.workflow_id.as_ref().map(ToString::to_string),
                    job.operation,
                    serde_json::to_string(&job.payload)?,
                    job.priority,
                    status_name(job.status),
                    job.idempotency_key,
                    serde_json::to_string(&job.depends_on)?,
                    job.attempts,
                    job.max_attempts,
                    job.run_at.to_rfc3339(),
                    job.deadline.map(|value| value.to_rfc3339()),
                    job.causation_id,
                    job.causation_depth,
                    job.created_at.to_rfc3339(),
                    job.updated_at.to_rfc3339(),
                ],
            )?;
            if inserted > 0 {
                crate::repository::outbox::insert(
                    tx,
                    &job.project_id.to_string(),
                    "job.enqueued",
                    &serde_json::json!({
                        "jobId": job.id.to_string(),
                        "operation": job.operation,
                    }),
                )?;
            }
            Ok(inserted > 0)
        })
    }

    pub fn next_runnable_job(&self, now: &str) -> Result<Option<Job>, StorageError> {
        let mut statement = self.connection.prepare(&format!(
            "SELECT {JOB_COLUMNS} FROM jobs
             WHERE status = 'pending' AND run_at <= ?1
             ORDER BY priority DESC, run_at ASC
             LIMIT 1"
        ))?;

        let row = statement.query_row([now], JobRow::from_row).optional()?;

        row.map(JobRow::into_job).transpose()
    }

    /// 原子领取下一个可执行任务：置为 running、attempts+1，
    /// 跳过依赖未完成的任务，并回收超过 stale_after 仍卡在 running 的陈旧任务。
    pub fn claim_next_job(
        &self,
        now: chrono::DateTime<Utc>,
        stale_after: chrono::Duration,
    ) -> Result<Option<Job>, StorageError> {
        let stale_cutoff = (now - stale_after).to_rfc3339();
        self.connection.execute(
            "UPDATE jobs SET status = 'pending', updated_at = ?1
             WHERE status = 'running' AND updated_at < ?2",
            params![now.to_rfc3339(), stale_cutoff],
        )?;

        let mut statement = self.connection.prepare(&format!(
            "SELECT {JOB_COLUMNS} FROM jobs
             WHERE status = 'pending' AND run_at <= ?1
             ORDER BY priority DESC, run_at ASC
             LIMIT 32"
        ))?;
        let candidates = statement
            .query_map([now.to_rfc3339()], JobRow::from_row)?
            .collect::<Result<Vec<JobRow>, _>>()?;

        for row in candidates {
            let job = row.into_job()?;
            if !self.dependencies_satisfied(&job)? {
                continue;
            }
            let claimed = self.connection.execute(
                "UPDATE jobs SET status = 'running', attempts = attempts + 1, updated_at = ?2
                 WHERE id = ?1 AND status = 'pending'",
                params![job.id.to_string(), now.to_rfc3339()],
            )?;
            if claimed == 1 {
                let mut claimed_job = job;
                claimed_job.status = JobStatus::Running;
                claimed_job.attempts += 1;
                return Ok(Some(claimed_job));
            }
            // 被并发领取则继续尝试下一个候选
        }
        Ok(None)
    }

    fn dependencies_satisfied(&self, job: &Job) -> Result<bool, StorageError> {
        for dep in &job.depends_on {
            let status: Option<String> = self
                .connection
                .query_row(
                    "SELECT status FROM jobs WHERE id = ?1",
                    [dep.to_string()],
                    |row| row.get(0),
                )
                .optional()?;
            if status.as_deref() != Some("succeeded") {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn complete_job(
        &self,
        job_id: &JobId,
        result: &serde_json::Value,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE jobs SET status = 'succeeded', result_json = ?2, error = NULL, updated_at = ?3
             WHERE id = ?1",
            params![
                job_id.to_string(),
                serde_json::to_string(result)?,
                now.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    /// 失败处理：达到 max_attempts 进死信，否则回到 pending 并按退避延后执行。
    /// 返回 true 表示已进入死信。
    pub fn fail_job(
        &self,
        job: &Job,
        error: &str,
        backoff: chrono::Duration,
        now: chrono::DateTime<Utc>,
    ) -> Result<bool, StorageError> {
        let dead = job.attempts >= job.max_attempts;
        if dead {
            self.connection.execute(
                "UPDATE jobs SET status = 'deadLetter', error = ?2, updated_at = ?3
                 WHERE id = ?1",
                params![job.id.to_string(), error, now.to_rfc3339()],
            )?;
        } else {
            let retry_at = now + backoff;
            self.connection.execute(
                "UPDATE jobs SET status = 'pending', error = ?2, run_at = ?4, updated_at = ?3
                 WHERE id = ?1",
                params![
                    job.id.to_string(),
                    error,
                    now.to_rfc3339(),
                    retry_at.to_rfc3339()
                ],
            )?;
        }
        Ok(dead)
    }

    pub fn list_jobs(&self, limit: u32) -> Result<Vec<Job>, StorageError> {
        let mut statement = self.connection.prepare(&format!(
            "SELECT {JOB_COLUMNS} FROM jobs ORDER BY created_at DESC LIMIT {limit}"
        ))?;
        let jobs = statement
            .query_map([], JobRow::from_row)?
            .collect::<Result<Vec<JobRow>, _>>()?;
        jobs.into_iter().map(JobRow::into_job).collect()
    }
}

fn status_name(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Pending => "pending",
        JobStatus::Blocked => "blocked",
        JobStatus::Running => "running",
        JobStatus::Succeeded => "succeeded",
        JobStatus::Failed => "failed",
        JobStatus::Cancelled => "cancelled",
        JobStatus::DeadLetter => "deadLetter",
    }
}

struct JobRow {
    id: String,
    project_id: String,
    workflow_id: Option<String>,
    operation: String,
    payload_json: String,
    priority: i32,
    status: String,
    idempotency_key: String,
    depends_on_json: String,
    attempts: u32,
    max_attempts: u32,
    run_at: String,
    deadline: Option<String>,
    causation_id: Option<String>,
    causation_depth: u32,
    created_at: String,
    updated_at: String,
}

impl JobRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(JobRow {
            id: row.get(0)?,
            project_id: row.get(1)?,
            workflow_id: row.get(2)?,
            operation: row.get(3)?,
            payload_json: row.get(4)?,
            priority: row.get(5)?,
            status: row.get(6)?,
            idempotency_key: row.get(7)?,
            depends_on_json: row.get(8)?,
            attempts: row.get(9)?,
            max_attempts: row.get(10)?,
            run_at: row.get(11)?,
            deadline: row.get(12)?,
            causation_id: row.get(13)?,
            causation_depth: row.get(14)?,
            created_at: row.get(15)?,
            updated_at: row.get(16)?,
        })
    }
}

impl JobRow {
    fn into_job(self) -> Result<Job, StorageError> {
        Ok(Job {
            id: Uuid::parse_str(&self.id)
                .map(novel_domain::JobId)
                .map_err(|_| DomainError::Validation("bad job id".into()))?,
            project_id: Uuid::parse_str(&self.project_id)
                .map(ProjectId)
                .map_err(|_| DomainError::Validation("bad project id".into()))?,
            workflow_id: self
                .workflow_id
                .map(|value| Uuid::parse_str(&value).map(novel_domain::WorkflowId))
                .transpose()
                .map_err(|_| DomainError::Validation("bad workflow id".into()))?,
            operation: self.operation,
            payload: serde_json::from_str(&self.payload_json)?,
            priority: self.priority,
            status: match self.status.as_str() {
                "pending" => JobStatus::Pending,
                "blocked" => JobStatus::Blocked,
                "running" => JobStatus::Running,
                "succeeded" => JobStatus::Succeeded,
                "failed" => JobStatus::Failed,
                "cancelled" => JobStatus::Cancelled,
                "deadLetter" => JobStatus::DeadLetter,
                _ => JobStatus::Failed,
            },
            idempotency_key: self.idempotency_key,
            depends_on: serde_json::from_str(&self.depends_on_json)?,
            attempts: self.attempts,
            max_attempts: self.max_attempts,
            run_at: chrono::DateTime::parse_from_rfc3339(&self.run_at)
                .map_err(|_| DomainError::Validation("bad run_at".into()))?
                .with_timezone(&chrono::Utc),
            deadline: self
                .deadline
                .map(|value| {
                    chrono::DateTime::parse_from_rfc3339(&value)
                        .map(|date| date.with_timezone(&chrono::Utc))
                        .map_err(|_| DomainError::Validation("bad deadline".into()))
                })
                .transpose()?,
            causation_id: self.causation_id,
            causation_depth: self.causation_depth,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .map_err(|_| DomainError::Validation("bad created_at".into()))?
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&self.updated_at)
                .map_err(|_| DomainError::Validation("bad updated_at".into()))?
                .with_timezone(&chrono::Utc),
        })
    }
}
