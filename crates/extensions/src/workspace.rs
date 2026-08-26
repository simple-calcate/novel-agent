//! 应用层：作品库编排、设置、队列入口、续写配置解析。
//!
//! 宿主只把 JSON 译成领域类型后调用这里。写库在 [`StorageHandle::with`]
//! 内完成，**返回之后**再 `kernel.dispatch`，订阅者可以再次进入 writer。

use crate::providers::resolve_provider_name;
use crate::util::{storage, with_repository};
use novel_domain::{
    Annotation, Book, BookId, Chapter, ChapterBody, ChapterId, ContentBlock, ContentPatch,
    DomainEvent, Job, JobId, JobStatus, JobView, LibrarySnapshot, Project, ProjectId, Revision,
};
use novel_kernel::{AgentSpec, DispatchSummary, Kernel, KernelError, ProviderConfig};
use novel_storage::{StorageError, SETTING_ACTIVE_PROJECT};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error(transparent)]
    Kernel(#[from] KernelError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("{0}")]
    Message(String),
}

/// 宿主面向的应用门面。不持有仓库锁跨越 `dispatch`。
pub struct Workspace<'a> {
    kernel: &'a Kernel,
}

impl<'a> Workspace<'a> {
    pub fn new(kernel: &'a Kernel) -> Self {
        Self { kernel }
    }

    pub fn kernel(&self) -> &'a Kernel {
        self.kernel
    }

    fn handle(&self) -> Result<std::sync::Arc<novel_storage::StorageHandle>, WorkspaceError> {
        Ok(storage(self.kernel)?)
    }

    fn dispatch_user(
        &self,
        event_type: &str,
        project_id: ProjectId,
        book_id: Option<BookId>,
        chapter_id: Option<ChapterId>,
        payload: Value,
    ) -> DispatchSummary {
        self.kernel.dispatch(&DomainEvent::user(
            event_type, project_id, book_id, chapter_id, payload,
        ))
    }

    pub fn create_project(&self, title: &str) -> Result<Project, WorkspaceError> {
        let project = self.handle()?.execute(|repository| {
            let project = repository.create_project(title)?;
            repository.save_setting(SETTING_ACTIVE_PROJECT, &project.id.to_string())?;
            Ok(project)
        })?;
        self.dispatch_user(
            "project.created",
            project.id.clone(),
            None,
            None,
            json!({ "title": project.title }),
        );
        Ok(project)
    }

    pub fn create_book(
        &self,
        project_id: &ProjectId,
        title: &str,
        synopsis: &str,
        position: u32,
    ) -> Result<Book, WorkspaceError> {
        let book = self
            .handle()?
            .execute(|repository| repository.create_book(project_id, title, synopsis, position))?;
        self.dispatch_user(
            "book.created",
            project_id.clone(),
            Some(book.id.clone()),
            None,
            json!({ "title": book.title, "position": book.position }),
        );
        Ok(book)
    }

    pub fn create_chapter(
        &self,
        project_id: &ProjectId,
        book_id: &str,
        title: &str,
        position: u32,
    ) -> Result<Chapter, WorkspaceError> {
        let chapter = self.handle()?.execute(|repository| {
            repository.create_chapter(project_id, book_id, title, position)
        })?;
        self.dispatch_user(
            "chapter.created",
            project_id.clone(),
            Some(chapter.book_id.clone()),
            Some(chapter.id.clone()),
            json!({
                "title": chapter.title,
                "bookId": chapter.book_id.to_string(),
                "position": chapter.position,
            }),
        );
        Ok(chapter)
    }

    pub fn load_library(
        &self,
        project_id: Option<ProjectId>,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        Ok(self.handle()?.execute(|repository| {
            let projects = repository.list_projects()?;
            let stored = repository
                .get_setting(SETTING_ACTIVE_PROJECT)
                .ok()
                .flatten();
            let active = project_id
                .map(|id| id.to_string())
                .or(stored)
                .or_else(|| projects.first().map(|project| project.id.to_string()));
            if let Some(id) = &active {
                let _ = repository.save_setting(SETTING_ACTIVE_PROJECT, id);
            }
            let (books, chapters) = if let Some(id) = &active {
                let pid: ProjectId = id.parse().map_err(|_| {
                    StorageError::Domain(novel_domain::DomainError::Validation(format!(
                        "invalid project id: {id}"
                    )))
                })?;
                (
                    repository.list_books(&pid)?,
                    repository.list_chapters(&pid)?,
                )
            } else {
                (Vec::new(), Vec::new())
            };
            Ok(LibrarySnapshot {
                projects,
                active_project_id: active,
                books,
                chapters,
            })
        })?)
    }

    pub fn set_active_project(
        &self,
        project_id: ProjectId,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.load_library(Some(project_id))
    }

    pub fn load_chapter(&self, chapter_id: &ChapterId) -> Result<ChapterBody, WorkspaceError> {
        Ok(self
            .handle()?
            .execute(|repository| read_chapter_body(repository, chapter_id))?)
    }

    pub fn save_chapter(
        &self,
        chapter_id: &ChapterId,
        text: &str,
        blocks: Option<Vec<ContentBlock>>,
    ) -> Result<ChapterBody, WorkspaceError> {
        Ok(self.handle()?.execute(|repository| {
            if let Some(blocks) = blocks {
                repository.save_block_sequence(chapter_id, &blocks)?;
            } else {
                repository.save_chapter_snapshot(chapter_id, text, "user")?;
            }
            read_chapter_body(repository, chapter_id)
        })?)
    }

    pub fn rename_project(
        &self,
        project_id: &ProjectId,
        title: &str,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.rename_project(project_id, title))?;
        self.dispatch_user(
            "project.renamed",
            project_id.clone(),
            None,
            None,
            json!({ "title": title }),
        );
        self.load_library(Some(project_id.clone()))
    }

    pub fn delete_project(
        &self,
        project_id: &ProjectId,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.delete_project(project_id))?;
        self.dispatch_user("project.deleted", project_id.clone(), None, None, json!({}));
        self.load_library(None)
    }

    pub fn rename_book(
        &self,
        project_id: &ProjectId,
        book_id: &BookId,
        title: &str,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.rename_book(project_id, book_id, title))?;
        self.dispatch_user(
            "book.renamed",
            project_id.clone(),
            Some(book_id.clone()),
            None,
            json!({ "title": title }),
        );
        self.load_library(Some(project_id.clone()))
    }

    pub fn delete_book(
        &self,
        project_id: &ProjectId,
        book_id: &BookId,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.delete_book(project_id, book_id))?;
        self.dispatch_user(
            "book.deleted",
            project_id.clone(),
            Some(book_id.clone()),
            None,
            json!({}),
        );
        self.load_library(Some(project_id.clone()))
    }

    pub fn move_book(
        &self,
        project_id: &ProjectId,
        book_id: &BookId,
        delta: i32,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.move_book(project_id, book_id, delta))?;
        self.dispatch_user(
            "book.reordered",
            project_id.clone(),
            Some(book_id.clone()),
            None,
            json!({ "delta": delta }),
        );
        self.load_library(Some(project_id.clone()))
    }

    pub fn rename_chapter(
        &self,
        project_id: &ProjectId,
        chapter_id: &ChapterId,
        title: &str,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.rename_chapter(project_id, chapter_id, title))?;
        self.dispatch_user(
            "chapter.renamed",
            project_id.clone(),
            None,
            Some(chapter_id.clone()),
            json!({ "title": title }),
        );
        self.load_library(Some(project_id.clone()))
    }

    pub fn delete_chapter(
        &self,
        project_id: &ProjectId,
        chapter_id: &ChapterId,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.delete_chapter(project_id, chapter_id))?;
        self.dispatch_user(
            "chapter.deleted",
            project_id.clone(),
            None,
            Some(chapter_id.clone()),
            json!({}),
        );
        self.load_library(Some(project_id.clone()))
    }

    pub fn move_chapter(
        &self,
        project_id: &ProjectId,
        chapter_id: &ChapterId,
        delta: i32,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.move_chapter(project_id, chapter_id, delta))?;
        self.dispatch_user(
            "chapter.reordered",
            project_id.clone(),
            None,
            Some(chapter_id.clone()),
            json!({ "delta": delta }),
        );
        self.load_library(Some(project_id.clone()))
    }

    pub fn save_annotation(&self, annotation: &Annotation) -> Result<(), WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.save_annotation(annotation))?;
        Ok(())
    }

    pub fn save_setting(&self, key: &str, value: &str) -> Result<(), WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.save_setting(key, value))?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, WorkspaceError> {
        Ok(self
            .handle()?
            .execute(|repository| repository.get_setting(key))?)
    }

    pub fn enqueue_job(
        &self,
        project_id: ProjectId,
        operation: String,
        payload: Value,
        priority: i32,
    ) -> Result<(JobId, bool), WorkspaceError> {
        let now = chrono::Utc::now();
        let job = Job {
            id: JobId::new(),
            project_id,
            workflow_id: None,
            operation,
            payload,
            priority,
            status: JobStatus::Pending,
            idempotency_key: format!("manual:{}", JobId::new()),
            depends_on: Vec::new(),
            attempts: 0,
            max_attempts: 3,
            run_at: now,
            deadline: None,
            causation_id: None,
            causation_depth: 0,
            created_at: now,
            updated_at: now,
        };
        let inserted = self
            .handle()?
            .execute(|repository| repository.enqueue_job(&job))?;
        Ok((job.id, inserted))
    }

    pub fn list_jobs(&self, limit: u32) -> Result<Vec<JobView>, WorkspaceError> {
        let jobs = self
            .handle()?
            .execute(|repository| repository.list_jobs(limit))?;
        Ok(jobs.iter().map(JobView::from_job).collect())
    }

    pub fn dispatch(&self, event: &DomainEvent) -> DispatchSummary {
        self.kernel.dispatch(event)
    }

    pub fn chapter_project_id(
        &self,
        chapter_id: &ChapterId,
    ) -> Result<Option<ProjectId>, WorkspaceError> {
        Ok(self
            .handle()?
            .execute(|repository| repository.chapter_project_id(chapter_id))?)
    }

    /// 解析续写用的 Provider：显式配置优先，否则读设置，缺 key 则 echo。
    pub fn resolve_provider_config(
        &self,
        override_config: Option<ProviderConfig>,
    ) -> Result<ProviderConfig, WorkspaceError> {
        if let Some(mut config) = override_config {
            if !config.api_key.is_empty() {
                config.provider = resolve_provider_name(&config);
                return Ok(config);
            }
        }
        Ok(load_provider_config_from_kernel(self.kernel)?)
    }

    pub async fn generate_continuation(
        &self,
        chapter_id: ChapterId,
        revision: Revision,
        prompt: String,
        context_text: String,
        override_config: Option<ProviderConfig>,
    ) -> Result<ContentPatch, WorkspaceError> {
        let provider_config = self.resolve_provider_config(override_config)?;
        let project_id = self.chapter_project_id(&chapter_id)?.unwrap_or_default();
        let spec = AgentSpec {
            id: Default::default(),
            project_id,
            chapter_id,
            base_revision: revision,
            prompt,
            context_text,
            budget: Default::default(),
            system_prompt: None,
            temperature: 0.8,
            emit_finish_event: true,
        };
        let report = self.kernel.run_continuation(&provider_config, spec).await?;
        Ok(report.patch)
    }
}

pub fn read_chapter_body(
    repository: &novel_storage::Repository,
    id: &ChapterId,
) -> Result<ChapterBody, StorageError> {
    let revision = repository.current_revision(id)?;
    let text = repository.chapter_text(id, revision)?.unwrap_or_default();
    let blocks = match repository.block_sequence(id, revision)? {
        Some(sequence) => sequence.blocks,
        None if text.is_empty() => Vec::new(),
        None => vec![ContentBlock {
            id: novel_domain::BlockId::new(),
            kind: novel_domain::BlockKind::Body,
            text: text.clone(),
            position: 0,
            markup: Vec::new(),
        }],
    };
    Ok(ChapterBody {
        chapter_id: id.to_string(),
        revision: revision.0,
        text,
        blocks,
    })
}

/// 从内核读取已保存的模型配置（无 ToolContext 时供应用层使用）。
pub fn load_provider_config_from_kernel(kernel: &Kernel) -> Result<ProviderConfig, KernelError> {
    let raw = with_repository(kernel, |repository| repository.get_setting("model_config"))?;
    let mut config = ProviderConfig::default();
    if let Some(raw) = raw {
        if let Ok(value) = serde_json::from_str::<Value>(&raw) {
            config.provider = value
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            config.api_key = value
                .get("apiKey")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            config.base_url = value
                .get("baseUrl")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            config.model = value
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
        }
    }
    config.provider = resolve_provider_name(&config);
    Ok(config)
}
