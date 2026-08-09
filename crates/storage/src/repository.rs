use crate::{run_migrations, StorageError};
use novel_domain::{
    Annotation, Chapter, ChapterId, ContentPatch, DomainError, DomainEvent, Job, JobStatus,
    Project, ProjectId, Revision, TextOperation, WorkflowRule,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use uuid::Uuid;

pub struct Repository {
    connection: Connection,
}

impl Repository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let mut connection = Connection::open(path)?;
        run_migrations(&mut connection)?;
        Ok(Self { connection })
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let mut connection = Connection::open_in_memory()?;
        run_migrations(&mut connection)?;
        Ok(Self { connection })
    }

    pub fn save_setting(&self, key: &str, value: &str) -> Result<(), StorageError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.connection.execute(
            "INSERT OR REPLACE INTO app_settings(key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, now],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StorageError> {
        let mut stmt = self
            .connection
            .prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    pub fn create_project(&self, title: &str) -> Result<Project, StorageError> {
        let now = chrono::Utc::now().to_rfc3339();
        let project = Project {
            id: ProjectId::new(),
            title: title.to_owned(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.connection.execute(
            "INSERT INTO projects(id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![project.id.to_string(), project.title, now, now],
        )?;
        Ok(project)
    }

    pub fn create_chapter(
        &self,
        project_id: &ProjectId,
        book_id: &str,
        title: &str,
        position: u32,
    ) -> Result<Chapter, StorageError> {
        let chapter = Chapter {
            id: ChapterId::new(),
            book_id: Uuid::parse_str(book_id).map(novel_domain::BookId).map_err(|_| {
                StorageError::Domain(DomainError::Validation("invalid book id".into()))
            })?,
            volume_id: None,
            title: title.to_owned(),
            position,
            current_revision: Revision::INITIAL,
            status: novel_domain::ChapterStatus::Draft,
        };

        self.connection.execute(
            "INSERT INTO chapters(id, book_id, title, position, current_revision, status)
             SELECT ?1, ?2, ?3, ?4, 0, 'draft'
             WHERE EXISTS(SELECT 1 FROM books WHERE id = ?2 AND project_id = ?5)",
            params![
                chapter.id.to_string(),
                book_id,
                title,
                position,
                project_id.to_string()
            ],
        )?;
        Ok(chapter)
    }

    pub fn current_revision(&self, chapter_id: &ChapterId) -> Result<Revision, StorageError> {
        let revision: Option<i64> = self
            .connection
            .query_row(
                "SELECT current_revision FROM chapters WHERE id = ?1",
                [chapter_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        revision
            .map(|value| Revision(value as u64))
            .ok_or_else(|| DomainError::NotFound(format!("chapter {chapter_id}")).into())
    }

    pub fn commit_patch(
        &mut self,
        patch: &ContentPatch,
        actor: &str,
        operations: &[TextOperation],
    ) -> Result<Revision, StorageError> {
        let transaction = self.connection.transaction()?;
        let actual: i64 = transaction.query_row(
            "SELECT current_revision FROM chapters WHERE id = ?1",
            [patch.chapter_id.to_string()],
            |row| row.get(0),
        )?;

        if actual as u64 != patch.base_revision.0 {
            return Err(DomainError::RevisionConflict {
                expected: patch.base_revision.0,
                actual: actual as u64,
            }
            .into());
        }

        let mut text: String = transaction
            .query_row(
                "SELECT text FROM revisions WHERE chapter_id = ?1 AND revision = ?2",
                params![patch.chapter_id.to_string(), actual],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or_default();

        for operation in operations {
            apply_operation(&mut text, operation)?;
        }

        let next = Revision((actual as u64) + 1);
        let now = chrono::Utc::now().to_rfc3339();
        transaction.execute(
            "INSERT INTO revisions(chapter_id, revision, format, text, created_at)
             VALUES (?1, ?2, 'plainText', ?3, ?4)",
            params![patch.chapter_id.to_string(), next.0 as i64, text, now],
        )?;
        transaction.execute(
            "UPDATE chapters SET current_revision = ?2 WHERE id = ?1",
            params![patch.chapter_id.to_string(), next.0 as i64],
        )?;
        transaction.execute(
            "INSERT INTO operation_log(
                project_id, chapter_id, revision_before, revision_after,
                actor, operations_json, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "",
                patch.chapter_id.to_string(),
                actual,
                next.0 as i64,
                actor,
                serde_json::to_string(operations)?,
                now,
            ],
        )?;
        transaction.commit()?;
        Ok(next)
    }

    pub fn record_event(&self, event: &DomainEvent) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO domain_events(
                id, event_type, schema_version, occurred_at, project_id, payload_json, event_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.event_id.to_string(),
                event.event_type,
                event.schema_version,
                event.occurred_at.to_rfc3339(),
                event.project_id.to_string(),
                serde_json::to_string(&event.payload)?,
                serde_json::to_string(event)?,
            ],
        )?;
        Ok(())
    }

    pub fn save_workflow(&self, rule: &WorkflowRule) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT OR REPLACE INTO workflows(
                id, project_id, name, enabled, priority, cooldown_ms, rule_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                rule.id.to_string(),
                rule.project_id.to_string(),
                rule.name,
                rule.enabled,
                rule.priority,
                rule.cooldown_ms as i64,
                serde_json::to_string(rule)?,
            ],
        )?;
        Ok(())
    }

    pub fn workflows_for_event(
        &self,
        project_id: &ProjectId,
        event_type: &str,
    ) -> Result<Vec<WorkflowRule>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT rule_json FROM workflows
             WHERE project_id = ?1 AND enabled = 1
             ORDER BY priority DESC",
        )?;
        let rows = statement.query_map([project_id.to_string()], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        let mut rules = Vec::new();
        for row in rows {
            let rule: WorkflowRule = serde_json::from_str(&row?)?;
            if rule.trigger.event_type == event_type {
                rules.push(rule);
            }
        }
        Ok(rules)
    }

    pub fn enqueue_job(&self, job: &Job) -> Result<bool, StorageError> {
        let inserted = self.connection.execute(
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
        Ok(inserted > 0)
    }

    pub fn next_runnable_job(&self, now: &str) -> Result<Option<Job>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, project_id, workflow_id, operation, payload_json, priority, status,
                    idempotency_key, depends_on_json, attempts, max_attempts, run_at,
                    deadline, causation_id, causation_depth, created_at, updated_at
             FROM jobs
             WHERE status = 'pending' AND run_at <= ?1
             ORDER BY priority DESC, run_at ASC
             LIMIT 1",
        )?;

        let row = statement
            .query_row([now], |row| {
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
            })
            .optional()?;

        row.map(JobRow::into_job).transpose()
    }

    pub fn save_annotation(&self, annotation: &Annotation) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT OR REPLACE INTO annotations(
                id, project_id, chapter_id, anchor_json, kind, body, resolved, outdated
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                annotation.id.to_string(),
                annotation.project_id.to_string(),
                annotation.chapter_id.to_string(),
                serde_json::to_string(&annotation.anchor)?,
                format!("{:?}", annotation.kind),
                annotation.body,
                annotation.resolved,
                annotation.outdated,
            ],
        )?;
        Ok(())
    }
}

pub fn apply_operation_for_test(
    text: &mut String,
    operation: &TextOperation,
) -> Result<(), StorageError> {
    apply_operation(text, operation)
}

fn apply_operation(text: &mut String, operation: &TextOperation) -> Result<(), StorageError> {
    match operation {
        TextOperation::Insert { offset, text: value, .. } => {
            let offset = (*offset as usize).min(text.len());
            text.insert_str(offset, value);
        }
        TextOperation::Delete { offset, length, .. } => {
            let start = (*offset as usize).min(text.len());
            let end = start.saturating_add(*length as usize).min(text.len());
            text.replace_range(start..end, "");
        }
        TextOperation::CreateBlock { text: value, .. } => {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(value);
        }
    }
    Ok(())
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
