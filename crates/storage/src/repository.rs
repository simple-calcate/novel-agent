use crate::{run_migrations, StorageError};
use novel_domain::{
    Actor, Annotation, BlockId, BlockKind, BlockSequence, Book, BookId, CanonEntity, CanonFact,
    Chapter, ChapterId, ChapterStatus, ContentBlock, ContentPatch, DomainError, DomainEvent,
    EntityKind, Job, JobId, JobStatus, PlotThread, Project, ProjectId, ProposalId, Revision,
    TextOperation, WorkflowId, WorkflowRule,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use uuid::Uuid;

use chrono::{DateTime, Utc};

pub const SETTING_ACTIVE_PROJECT: &str = "active_project_id";

const JOB_COLUMNS: &str = "id, project_id, workflow_id, operation, payload_json, priority, status,
                    idempotency_key, depends_on_json, attempts, max_attempts, run_at,
                    deadline, causation_id, causation_depth, created_at, updated_at";

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

    pub fn create_book(
        &self,
        project_id: &ProjectId,
        title: &str,
        synopsis: &str,
        position: u32,
    ) -> Result<Book, StorageError> {
        let position = if position == 0 {
            self.next_position(
                "SELECT COALESCE(MAX(position), 0) FROM books WHERE project_id = ?1",
                &project_id.to_string(),
            )?
        } else {
            position
        };
        let book = Book {
            id: BookId::new(),
            project_id: project_id.clone(),
            title: title.to_owned(),
            synopsis: synopsis.to_owned(),
            position,
        };
        let inserted = self.connection.execute(
            "INSERT INTO books(id, project_id, title, synopsis, position)
             SELECT ?1, ?2, ?3, ?4, ?5
             WHERE EXISTS(SELECT 1 FROM projects WHERE id = ?2)",
            params![
                book.id.to_string(),
                project_id.to_string(),
                book.title,
                book.synopsis,
                position
            ],
        )?;
        if inserted == 0 {
            return Err(DomainError::NotFound(format!("project {project_id}")).into());
        }
        Ok(book)
    }

    pub fn create_chapter(
        &self,
        project_id: &ProjectId,
        book_id: &str,
        title: &str,
        position: u32,
    ) -> Result<Chapter, StorageError> {
        let position = if position == 0 {
            self.next_position(
                "SELECT COALESCE(MAX(position), 0) FROM chapters WHERE book_id = ?1",
                book_id,
            )?
        } else {
            position
        };
        let chapter = Chapter {
            id: ChapterId::new(),
            book_id: Uuid::parse_str(book_id).map(BookId).map_err(|_| {
                StorageError::Domain(DomainError::Validation("invalid book id".into()))
            })?,
            volume_id: None,
            title: title.to_owned(),
            position,
            current_revision: Revision::INITIAL,
            status: ChapterStatus::Draft,
        };

        let inserted = self.connection.execute(
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
        if inserted == 0 {
            return Err(
                DomainError::NotFound(format!("book {book_id} in project {project_id}")).into(),
            );
        }
        Ok(chapter)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, title, created_at, updated_at FROM projects ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut projects = Vec::new();
        for row in rows {
            let (id, title, created_at, updated_at) = row?;
            projects.push(Project {
                id: parse_project_id(&id)?,
                title,
                created_at: parse_rfc3339(&created_at),
                updated_at: parse_rfc3339(&updated_at),
            });
        }
        Ok(projects)
    }

    pub fn list_books(&self, project_id: &ProjectId) -> Result<Vec<Book>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, project_id, title, synopsis, position
             FROM books WHERE project_id = ?1 ORDER BY position, title",
        )?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?;
        let mut books = Vec::new();
        for row in rows {
            let (id, project, title, synopsis, position) = row?;
            books.push(Book {
                id: parse_book_id(&id)?,
                project_id: parse_project_id(&project)?,
                title,
                synopsis,
                position: position as u32,
            });
        }
        Ok(books)
    }

    pub fn list_chapters(&self, project_id: &ProjectId) -> Result<Vec<Chapter>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT c.id, c.book_id, c.volume_id, c.title, c.position, c.current_revision, c.status
             FROM chapters c
             JOIN books b ON b.id = c.book_id
             WHERE b.project_id = ?1
             ORDER BY b.position, c.position, c.title",
        )?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        let mut chapters = Vec::new();
        for row in rows {
            let (id, book_id, volume_id, title, position, revision, status) = row?;
            chapters.push(Chapter {
                id: id.parse().map_err(|_| {
                    StorageError::Domain(DomainError::Validation("invalid chapter id".into()))
                })?,
                book_id: parse_book_id(&book_id)?,
                volume_id: volume_id
                    .map(|value| {
                        value.parse().map_err(|_| {
                            StorageError::Domain(DomainError::Validation(
                                "invalid volume id".into(),
                            ))
                        })
                    })
                    .transpose()?,
                title,
                position: position as u32,
                current_revision: Revision(revision as u64),
                status: chapter_status(&status),
            });
        }
        Ok(chapters)
    }

    pub fn rename_project(
        &self,
        project_id: &ProjectId,
        title: &str,
    ) -> Result<Project, StorageError> {
        let now = Utc::now().to_rfc3339();
        let updated = self.connection.execute(
            "UPDATE projects SET title = ?2, updated_at = ?3 WHERE id = ?1",
            params![project_id.to_string(), title, now],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound(format!("project {project_id}")).into());
        }
        self.list_projects()?
            .into_iter()
            .find(|project| &project.id == project_id)
            .ok_or_else(|| DomainError::NotFound(format!("project {project_id}")).into())
    }

    pub fn delete_project(&self, project_id: &ProjectId) -> Result<(), StorageError> {
        self.connection.execute(
            "DELETE FROM jobs WHERE project_id = ?1",
            [project_id.to_string()],
        )?;
        self.connection.execute(
            "DELETE FROM domain_events WHERE project_id = ?1",
            [project_id.to_string()],
        )?;
        self.connection.execute(
            "DELETE FROM workflows WHERE project_id = ?1",
            [project_id.to_string()],
        )?;
        let deleted = self.connection.execute(
            "DELETE FROM projects WHERE id = ?1",
            [project_id.to_string()],
        )?;
        if deleted == 0 {
            return Err(DomainError::NotFound(format!("project {project_id}")).into());
        }
        if let Ok(Some(active)) = self.get_setting(SETTING_ACTIVE_PROJECT) {
            if active == project_id.to_string() {
                let _ = self.connection.execute(
                    "DELETE FROM app_settings WHERE key = ?1",
                    [SETTING_ACTIVE_PROJECT],
                );
            }
        }
        Ok(())
    }

    pub fn rename_book(
        &self,
        project_id: &ProjectId,
        book_id: &BookId,
        title: &str,
    ) -> Result<Book, StorageError> {
        let updated = self.connection.execute(
            "UPDATE books SET title = ?3 WHERE id = ?1 AND project_id = ?2",
            params![book_id.to_string(), project_id.to_string(), title],
        )?;
        if updated == 0 {
            return Err(
                DomainError::NotFound(format!("book {book_id} in project {project_id}")).into(),
            );
        }
        self.list_books(project_id)?
            .into_iter()
            .find(|book| &book.id == book_id)
            .ok_or_else(|| DomainError::NotFound(format!("book {book_id}")).into())
    }

    pub fn delete_book(
        &self,
        project_id: &ProjectId,
        book_id: &BookId,
    ) -> Result<(), StorageError> {
        let deleted = self.connection.execute(
            "DELETE FROM books WHERE id = ?1 AND project_id = ?2",
            params![book_id.to_string(), project_id.to_string()],
        )?;
        if deleted == 0 {
            return Err(
                DomainError::NotFound(format!("book {book_id} in project {project_id}")).into(),
            );
        }
        Ok(())
    }

    pub fn move_book(
        &self,
        project_id: &ProjectId,
        book_id: &BookId,
        delta: i32,
    ) -> Result<Vec<Book>, StorageError> {
        let mut books = self.list_books(project_id)?;
        let Some(index) = books.iter().position(|book| &book.id == book_id) else {
            return Err(
                DomainError::NotFound(format!("book {book_id} in project {project_id}")).into(),
            );
        };
        let target = index as i32 + delta;
        if target >= 0 && (target as usize) < books.len() {
            books.swap(index, target as usize);
            self.write_book_positions(project_id, &books)?;
        }
        self.list_books(project_id)
    }

    pub fn rename_chapter(
        &self,
        project_id: &ProjectId,
        chapter_id: &ChapterId,
        title: &str,
    ) -> Result<Chapter, StorageError> {
        let updated = self.connection.execute(
            "UPDATE chapters SET title = ?2
             WHERE id = ?1 AND book_id IN (SELECT id FROM books WHERE project_id = ?3)",
            params![chapter_id.to_string(), title, project_id.to_string()],
        )?;
        if updated == 0 {
            return Err(DomainError::NotFound(format!("chapter {chapter_id}")).into());
        }
        self.list_chapters(project_id)?
            .into_iter()
            .find(|chapter| &chapter.id == chapter_id)
            .ok_or_else(|| DomainError::NotFound(format!("chapter {chapter_id}")).into())
    }

    pub fn delete_chapter(
        &self,
        project_id: &ProjectId,
        chapter_id: &ChapterId,
    ) -> Result<(), StorageError> {
        let deleted = self.connection.execute(
            "DELETE FROM chapters
             WHERE id = ?1 AND book_id IN (SELECT id FROM books WHERE project_id = ?2)",
            params![chapter_id.to_string(), project_id.to_string()],
        )?;
        if deleted == 0 {
            return Err(DomainError::NotFound(format!("chapter {chapter_id}")).into());
        }
        Ok(())
    }

    pub fn move_chapter(
        &self,
        project_id: &ProjectId,
        chapter_id: &ChapterId,
        delta: i32,
    ) -> Result<Vec<Chapter>, StorageError> {
        let chapters = self.list_chapters(project_id)?;
        let Some(chapter) = chapters.iter().find(|item| &item.id == chapter_id) else {
            return Err(DomainError::NotFound(format!("chapter {chapter_id}")).into());
        };
        let book_id = chapter.book_id.clone();
        let mut siblings: Vec<Chapter> = chapters
            .into_iter()
            .filter(|item| item.book_id == book_id)
            .collect();
        let Some(index) = siblings.iter().position(|item| &item.id == chapter_id) else {
            return Err(DomainError::NotFound(format!("chapter {chapter_id}")).into());
        };
        let target = index as i32 + delta;
        if target >= 0 && (target as usize) < siblings.len() {
            siblings.swap(index, target as usize);
            for (position, item) in siblings.iter().enumerate() {
                self.connection.execute(
                    "UPDATE chapters SET position = ?2 WHERE id = ?1",
                    params![item.id.to_string(), (position as u32) + 1],
                )?;
            }
        }
        self.list_chapters(project_id)
    }

    fn write_book_positions(
        &self,
        project_id: &ProjectId,
        books: &[Book],
    ) -> Result<(), StorageError> {
        for (index, book) in books.iter().enumerate() {
            self.connection.execute(
                "UPDATE books SET position = ?2 WHERE id = ?1 AND project_id = ?3",
                params![
                    book.id.to_string(),
                    (index as u32) + 1,
                    project_id.to_string()
                ],
            )?;
        }
        Ok(())
    }

    /// 提交块序列；内容（忽略块 id）未变则不升版本。
    pub fn save_block_sequence(
        &mut self,
        chapter_id: &ChapterId,
        blocks: &[ContentBlock],
    ) -> Result<Revision, StorageError> {
        let base = self.current_revision(chapter_id)?;
        if let Some(existing) = self.block_sequence(chapter_id, base)? {
            if blocks_content_eq(&existing.blocks, blocks) {
                return Ok(base);
            }
        } else if blocks.is_empty() {
            let text = self.chapter_text(chapter_id, base)?.unwrap_or_default();
            if text.is_empty() {
                return Ok(base);
            }
        }
        self.commit_block_sequence(chapter_id, base, blocks)
    }

    /// 把当前正文写成新版本。文本未变则不递增修订号。
    pub fn save_chapter_snapshot(
        &mut self,
        chapter_id: &ChapterId,
        text: &str,
        actor: &str,
    ) -> Result<Revision, StorageError> {
        let base = self.current_revision(chapter_id)?;
        let current = self.chapter_text(chapter_id, base)?.unwrap_or_default();
        if current == text {
            return Ok(base);
        }
        let block_id = BlockId::new();
        let mut operations = Vec::new();
        if !current.is_empty() {
            operations.push(TextOperation::Delete {
                block_id: block_id.clone(),
                offset: 0,
                length: current.len() as u32,
            });
        }
        if !text.is_empty() {
            operations.push(TextOperation::Insert {
                block_id,
                offset: 0,
                text: text.to_owned(),
            });
        }
        let patch = ContentPatch {
            id: ProposalId::new(),
            chapter_id: chapter_id.clone(),
            base_revision: base,
            operations: operations.clone(),
            rationale: "editor snapshot".into(),
            created_by: Actor::User { user_id: None },
            created_at: Utc::now(),
        };
        self.commit_patch(&patch, actor, &operations)
    }

    fn next_position(&self, sql: &str, key: &str) -> Result<u32, StorageError> {
        let max: i64 = self.connection.query_row(sql, [key], |row| row.get(0))?;
        Ok((max as u32).saturating_add(1))
    }

    /// 章节 → 所属项目（operation_log 等处需要真实 project_id）。
    pub fn chapter_project_id(
        &self,
        chapter_id: &ChapterId,
    ) -> Result<Option<ProjectId>, StorageError> {
        let project_id: Option<String> = self
            .connection
            .query_row(
                "SELECT b.project_id FROM books b
                 JOIN chapters c ON c.book_id = b.id
                 WHERE c.id = ?1",
                [chapter_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        project_id
            .map(|value| {
                Uuid::parse_str(&value)
                    .map(ProjectId)
                    .map_err(|_| DomainError::Validation("bad project id".into()).into())
            })
            .transpose()
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

    /// 读取某个历史版本的正文字本。
    pub fn chapter_text(
        &self,
        chapter_id: &ChapterId,
        revision: Revision,
    ) -> Result<Option<String>, StorageError> {
        let text: Option<String> = self
            .connection
            .query_row(
                "SELECT text FROM revisions WHERE chapter_id = ?1 AND revision = ?2",
                params![chapter_id.to_string(), revision.0 as i64],
                |row| row.get(0),
            )
            .optional()?;
        Ok(text)
    }

    /// 某章节全部操作日志的 project_id（校验日志归属用）。
    pub fn operation_log_project_ids(
        &self,
        chapter_id: &ChapterId,
    ) -> Result<Vec<String>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT project_id FROM operation_log WHERE chapter_id = ?1 ORDER BY id")?;
        let rows = statement.query_map([chapter_id.to_string()], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
        let project_id: Option<String> = transaction
            .query_row(
                "SELECT b.project_id FROM books b
                 JOIN chapters c ON c.book_id = b.id
                 WHERE c.id = ?1",
                [patch.chapter_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;
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
                project_id.unwrap_or_default(),
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

    /// 工作流冷却检查：cooldown_ms 内已触发过则不再触发。
    pub fn workflow_in_cooldown(
        &self,
        rule: &WorkflowRule,
        event_type: &str,
        now: chrono::DateTime<Utc>,
    ) -> Result<bool, StorageError> {
        if rule.cooldown_ms == 0 {
            return Ok(false);
        }
        let last: Option<String> = self
            .connection
            .query_row(
                "SELECT last_fired_at FROM workflow_fired
                 WHERE workflow_id = ?1 AND event_type = ?2",
                params![rule.id.to_string(), event_type],
                |row| row.get(0),
            )
            .optional()?;
        let Some(last) = last else {
            return Ok(false);
        };
        let last = chrono::DateTime::parse_from_rfc3339(&last)
            .map_err(|_| DomainError::Validation("bad last_fired_at".into()))?
            .with_timezone(&chrono::Utc);
        Ok(now - last < chrono::Duration::milliseconds(rule.cooldown_ms as i64))
    }

    pub fn record_workflow_fired(
        &self,
        workflow_id: &WorkflowId,
        event_type: &str,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT OR REPLACE INTO workflow_fired(workflow_id, event_type, last_fired_at)
             VALUES (?1, ?2, ?3)",
            params![workflow_id.to_string(), event_type, now.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn list_canon_entities(&self) -> Result<Vec<CanonEntity>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, branch_id, kind, canonical_name, aliases_json, attributes_json
             FROM canon_entities",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        let mut entities = Vec::new();
        for row in rows {
            let (id, branch_id, kind, canonical_name, aliases, attributes) = row?;
            let Ok(kind) = serde_json::from_value::<EntityKind>(serde_json::Value::String(kind))
            else {
                continue;
            };
            let Some(id) = id.parse().ok() else {
                continue;
            };
            entities.push(CanonEntity {
                id,
                branch_id,
                kind,
                canonical_name,
                aliases: serde_json::from_str(&aliases).unwrap_or_default(),
                attributes: serde_json::from_str(&attributes).unwrap_or_default(),
            });
        }
        Ok(entities)
    }

    pub fn list_canon_facts(&self) -> Result<Vec<CanonFact>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT fact_json FROM canon_facts")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut facts = Vec::new();
        for row in rows {
            if let Ok(fact) = serde_json::from_str::<CanonFact>(&row?) {
                facts.push(fact);
            }
        }
        Ok(facts)
    }

    pub fn list_plot_threads(&self) -> Result<Vec<PlotThread>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT data_json FROM plot_threads")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut threads = Vec::new();
        for row in rows {
            if let Ok(thread) = serde_json::from_str::<PlotThread>(&row?) {
                threads.push(thread);
            }
        }
        Ok(threads)
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

    pub fn save_result_object(
        &self,
        project_id: &ProjectId,
        content_type: &str,
        content: &str,
    ) -> Result<novel_domain::ResultObjectId, StorageError> {
        let id = novel_domain::ResultObjectId::new();
        self.connection.execute(
            "INSERT INTO result_objects(id, project_id, content_type, content, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id.to_string(),
                project_id.to_string(),
                content_type,
                content,
                chrono::Utc::now().to_rfc3339()
            ],
        )?;
        Ok(id)
    }

    /// 用正史实体重建 FTS 检索索引，返回索引条数。
    pub fn rebuild_search_index(&self, project_id: &ProjectId) -> Result<u32, StorageError> {
        self.connection.execute(
            "DELETE FROM search_documents WHERE project_id = ?1",
            params![project_id.to_string()],
        )?;
        let inserted = self.connection.execute(
            "INSERT INTO search_documents(project_id, entity_kind, entity_id, title, body)
             SELECT ?1, kind, id, canonical_name, canonical_name || ' ' || aliases_json
             FROM canon_entities",
            params![project_id.to_string()],
        )?;
        Ok(inserted as u32)
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

    /// 提交块序列：校验 base_revision 冲突，写入新版本的块行，
    /// 同步纯文本快照（正文拼接）以兼容旧读取路径，更新章节版本并登记操作日志。
    pub fn commit_block_sequence(
        &mut self,
        chapter_id: &ChapterId,
        base_revision: Revision,
        blocks: &[ContentBlock],
    ) -> Result<Revision, StorageError> {
        let transaction = self.connection.transaction()?;
        let actual: i64 = transaction.query_row(
            "SELECT current_revision FROM chapters WHERE id = ?1",
            [chapter_id.to_string()],
            |row| row.get(0),
        )?;

        if actual as u64 != base_revision.0 {
            return Err(DomainError::RevisionConflict {
                expected: base_revision.0,
                actual: actual as u64,
            }
            .into());
        }

        let next = Revision((actual as u64) + 1);
        let now = chrono::Utc::now().to_rfc3339();

        transaction.execute(
            "DELETE FROM content_blocks WHERE chapter_id = ?1 AND revision = ?2",
            params![chapter_id.to_string(), next.0 as i64],
        )?;

        for block in blocks {
            transaction.execute(
                "INSERT INTO content_blocks(
                    id, chapter_id, revision, kind, position, text, markup_json, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    block.id.to_string(),
                    chapter_id.to_string(),
                    next.0 as i64,
                    match block.kind {
                        BlockKind::Body => "body",
                        BlockKind::Thinking => "thinking",
                    },
                    block.position,
                    block.text,
                    serde_json::to_string(&block.markup)?,
                    now,
                ],
            )?;
        }

        // 纯文本快照：正文拼接，兼容旧读取路径（chapter_text / DocumentSnapshot）。
        let body_text = blocks
            .iter()
            .filter(|block| block.kind == BlockKind::Body)
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        transaction.execute(
            "INSERT INTO revisions(chapter_id, revision, format, text, created_at)
             VALUES (?1, ?2, 'structuredAst', ?3, ?4)",
            params![chapter_id.to_string(), next.0 as i64, body_text, now],
        )?;
        transaction.execute(
            "UPDATE chapters SET current_revision = ?2 WHERE id = ?1",
            params![chapter_id.to_string(), next.0 as i64],
        )?;

        let project_id: Option<String> = transaction
            .query_row(
                "SELECT b.project_id FROM books b
                 JOIN chapters c ON c.book_id = b.id
                 WHERE c.id = ?1",
                [chapter_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        transaction.execute(
            "INSERT INTO operation_log(
                project_id, chapter_id, revision_before, revision_after,
                actor, operations_json, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                project_id.unwrap_or_default(),
                chapter_id.to_string(),
                actual,
                next.0 as i64,
                "editor",
                serde_json::to_string(&serde_json::json!({
                    "operation": "commitBlockSequence",
                    "blockCount": blocks.len(),
                    "blocks": blocks,
                }))?,
                now,
            ],
        )?;

        transaction.commit()?;
        Ok(next)
    }

    /// 读取指定版本的块序列；该版本无块数据时返回 None。
    pub fn block_sequence(
        &self,
        chapter_id: &ChapterId,
        revision: Revision,
    ) -> Result<Option<BlockSequence>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, kind, position, text, markup_json FROM content_blocks
             WHERE chapter_id = ?1 AND revision = ?2
             ORDER BY position ASC",
        )?;
        let rows =
            statement.query_map(params![chapter_id.to_string(), revision.0 as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, u32>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;

        let mut blocks = Vec::new();
        for row in rows {
            let (id, kind, position, text, markup_json) = row?;
            let Some(id) = id.parse().ok() else {
                continue;
            };
            let kind = match kind.as_str() {
                "body" => BlockKind::Body,
                "thinking" => BlockKind::Thinking,
                _ => continue,
            };
            blocks.push(ContentBlock {
                id,
                kind,
                text,
                position,
                markup: serde_json::from_str(&markup_json).unwrap_or_default(),
            });
        }

        if blocks.is_empty() {
            return Ok(None);
        }

        let created_at: Option<String> = self
            .connection
            .query_row(
                "SELECT created_at FROM revisions WHERE chapter_id = ?1 AND revision = ?2",
                params![chapter_id.to_string(), revision.0 as i64],
                |row| row.get(0),
            )
            .optional()?;
        let created_at = created_at
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
            .map(|date| date.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        Ok(Some(BlockSequence {
            chapter_id: chapter_id.clone(),
            revision,
            blocks,
            created_at,
        }))
    }

    /// 读取章节最新版本的块序列。
    pub fn latest_block_sequence(
        &self,
        chapter_id: &ChapterId,
    ) -> Result<Option<BlockSequence>, StorageError> {
        let revision = self.current_revision(chapter_id)?;
        self.block_sequence(chapter_id, revision)
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
        TextOperation::Insert {
            offset,
            text: value,
            ..
        } => {
            // 偏移按字节解释，但必须对齐到字符边界，否则 UTF-8 中间会 panic。
            let offset = floor_char_boundary(text, *offset as usize);
            text.insert_str(offset, value);
        }
        TextOperation::Delete { offset, length, .. } => {
            let start = floor_char_boundary(text, *offset as usize);
            let end = floor_char_boundary(text, start.saturating_add(*length as usize));
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

/// 把字节下标对齐到不大于它的字符边界（`str::floor_char_boundary` 的稳定实现）。
fn floor_char_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
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

fn parse_rfc3339(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn parse_project_id(value: &str) -> Result<ProjectId, StorageError> {
    value
        .parse()
        .map_err(|_| DomainError::Validation("invalid project id".into()).into())
}

fn parse_book_id(value: &str) -> Result<BookId, StorageError> {
    value
        .parse()
        .map_err(|_| DomainError::Validation("invalid book id".into()).into())
}

fn chapter_status(name: &str) -> ChapterStatus {
    match name {
        "completed" => ChapterStatus::Completed,
        "archived" => ChapterStatus::Archived,
        _ => ChapterStatus::Draft,
    }
}

fn blocks_content_eq(left: &[ContentBlock], right: &[ContentBlock]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(a, b)| {
            a.kind == b.kind && a.text == b.text && a.position == b.position && a.markup == b.markup
        })
}
