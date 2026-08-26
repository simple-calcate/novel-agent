//! 应用层：作品库编排、设置、队列入口、续写配置解析。
//!
//! 宿主只把 JSON 译成领域类型后调用这里。写库在 [`StorageHandle::with`]
//! 内完成，**返回之后**再 `kernel.dispatch`，订阅者可以再次进入 writer。

use crate::providers::resolve_provider_name;
use crate::secrets::{SecretVault, MODEL_API_KEY};
use crate::util::{storage, with_repository};
use novel_domain::{
    Annotation, Book, BookId, CanonProposal, Chapter, ChapterBody, ChapterId, ContentBlock,
    ContentPatch, DomainEvent, FactId, FactStatus, Job, JobId, JobStatus, JobView, LibrarySnapshot,
    PluginSummary, PreferenceRule, PreferenceRuleId, PreferenceScope, PreferenceStatus, Project,
    ProjectId, ProposalId, RejectionReason, Revision, Scene, SceneId, StoryEntry, StoryEntryKind,
    Volume, VolumeId,
};
use novel_kernel::{AgentSpec, DispatchSummary, Kernel, KernelError, ProviderConfig};
use novel_storage::{StorageError, SETTING_ACTIVE_PROJECT};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error(transparent)]
    Kernel(#[from] KernelError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Secret(#[from] crate::secrets::SecretError),
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfigView {
    pub provider: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_key_set: bool,
    pub base_url: String,
    pub model: String,
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
        volume_id: Option<&str>,
    ) -> Result<Chapter, WorkspaceError> {
        let chapter = self.handle()?.execute(|repository| {
            repository.create_chapter_with_volume(project_id, book_id, title, position, volume_id)
        })?;
        self.dispatch_user(
            "chapter.created",
            project_id.clone(),
            Some(chapter.book_id.clone()),
            Some(chapter.id.clone()),
            json!({
                "title": chapter.title,
                "bookId": chapter.book_id.to_string(),
                "volumeId": chapter.volume_id.as_ref().map(ToString::to_string),
                "position": chapter.position,
            }),
        );
        Ok(chapter)
    }

    pub fn create_volume(
        &self,
        project_id: &ProjectId,
        book_id: &str,
        title: &str,
        position: u32,
    ) -> Result<Volume, WorkspaceError> {
        let volume = self
            .handle()?
            .execute(|repository| repository.create_volume(project_id, book_id, title, position))?;
        self.dispatch_user(
            "volume.created",
            project_id.clone(),
            Some(volume.book_id.clone()),
            None,
            json!({
                "volumeId": volume.id.to_string(),
                "title": volume.title,
                "position": volume.position,
            }),
        );
        Ok(volume)
    }

    pub fn create_scene(
        &self,
        project_id: &ProjectId,
        chapter_id: &str,
        title: &str,
        position: u32,
        pov_entry_id: Option<&str>,
    ) -> Result<Scene, WorkspaceError> {
        let scene = self.handle()?.execute(|repository| {
            repository.create_scene(project_id, chapter_id, title, position, pov_entry_id)
        })?;
        self.dispatch_user(
            "scene.created",
            project_id.clone(),
            None,
            Some(scene.chapter_id.clone()),
            json!({
                "sceneId": scene.id.to_string(),
                "title": scene.title,
                "position": scene.position,
                "povEntryId": scene.pov_entry_id,
            }),
        );
        Ok(scene)
    }

    pub fn rename_scene(
        &self,
        project_id: &ProjectId,
        scene_id: &SceneId,
        title: &str,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.rename_scene(project_id, scene_id, title))?;
        self.dispatch_user(
            "scene.renamed",
            project_id.clone(),
            None,
            None,
            json!({ "sceneId": scene_id.to_string(), "title": title }),
        );
        self.load_library(Some(project_id.clone()))
    }

    pub fn set_scene_pov(
        &self,
        project_id: &ProjectId,
        scene_id: &SceneId,
        pov_entry_id: Option<&str>,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.set_scene_pov(project_id, scene_id, pov_entry_id))?;
        self.dispatch_user(
            "scene.updated",
            project_id.clone(),
            None,
            None,
            json!({ "sceneId": scene_id.to_string(), "povEntryId": pov_entry_id }),
        );
        self.load_library(Some(project_id.clone()))
    }

    pub fn delete_scene(
        &self,
        project_id: &ProjectId,
        scene_id: &SceneId,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.delete_scene(project_id, scene_id))?;
        self.dispatch_user(
            "scene.deleted",
            project_id.clone(),
            None,
            None,
            json!({ "sceneId": scene_id.to_string() }),
        );
        self.load_library(Some(project_id.clone()))
    }

    pub fn move_scene(
        &self,
        project_id: &ProjectId,
        scene_id: &SceneId,
        delta: i32,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.move_scene(project_id, scene_id, delta))?;
        self.dispatch_user(
            "scene.reordered",
            project_id.clone(),
            None,
            None,
            json!({ "sceneId": scene_id.to_string(), "delta": delta }),
        );
        self.load_library(Some(project_id.clone()))
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
            let (books, volumes, chapters, scenes) = if let Some(id) = &active {
                let pid: ProjectId = id.parse().map_err(|_| {
                    StorageError::Domain(novel_domain::DomainError::Validation(format!(
                        "invalid project id: {id}"
                    )))
                })?;
                (
                    repository.list_books(&pid)?,
                    repository.list_volumes(&pid)?,
                    repository.list_chapters(&pid)?,
                    repository.list_scenes(&pid)?,
                )
            } else {
                (Vec::new(), Vec::new(), Vec::new(), Vec::new())
            };
            Ok(LibrarySnapshot {
                projects,
                active_project_id: active,
                books,
                volumes,
                chapters,
                scenes,
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

    pub fn rename_volume(
        &self,
        project_id: &ProjectId,
        volume_id: &VolumeId,
        title: &str,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.rename_volume(project_id, volume_id, title))?;
        self.dispatch_user(
            "volume.renamed",
            project_id.clone(),
            None,
            None,
            json!({ "volumeId": volume_id.to_string(), "title": title }),
        );
        self.load_library(Some(project_id.clone()))
    }

    pub fn delete_volume(
        &self,
        project_id: &ProjectId,
        volume_id: &VolumeId,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.delete_volume(project_id, volume_id))?;
        self.dispatch_user(
            "volume.deleted",
            project_id.clone(),
            None,
            None,
            json!({ "volumeId": volume_id.to_string() }),
        );
        self.load_library(Some(project_id.clone()))
    }

    pub fn move_volume(
        &self,
        project_id: &ProjectId,
        volume_id: &VolumeId,
        delta: i32,
    ) -> Result<LibrarySnapshot, WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.move_volume(project_id, volume_id, delta))?;
        self.dispatch_user(
            "volume.reordered",
            project_id.clone(),
            None,
            None,
            json!({ "volumeId": volume_id.to_string(), "delta": delta }),
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

    pub fn save_model_config(
        &self,
        provider: &str,
        api_key: &str,
        base_url: &str,
        model: &str,
    ) -> Result<(), WorkspaceError> {
        let stored = json!({
            "provider": provider,
            "baseUrl": base_url,
            "model": model,
        });
        self.save_setting("model_config", &stored.to_string())?;
        if !api_key.is_empty() {
            self.vault()?.put(MODEL_API_KEY, api_key)?;
        }
        Ok(())
    }

    pub fn load_model_config(&self) -> Result<Option<ModelConfigView>, WorkspaceError> {
        let Some(raw) = self.get_setting("model_config")? else {
            return Ok(None);
        };
        let value: Value = serde_json::from_str(&raw)
            .map_err(|error| WorkspaceError::Message(error.to_string()))?;
        let leftover_key = value
            .get("apiKey")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if !leftover_key.is_empty() {
            if let Ok(vault) = self.vault() {
                vault.put(MODEL_API_KEY, &leftover_key)?;
                let stripped = json!({
                    "provider": value.get("provider").and_then(Value::as_str).unwrap_or_default(),
                    "baseUrl": value.get("baseUrl").and_then(Value::as_str).unwrap_or_default(),
                    "model": value.get("model").and_then(Value::as_str).unwrap_or_default(),
                });
                self.save_setting("model_config", &stripped.to_string())?;
            }
        }
        let api_key_set = self
            .vault()
            .ok()
            .map(|vault| vault.is_set(MODEL_API_KEY))
            .unwrap_or(!leftover_key.is_empty());
        Ok(Some(ModelConfigView {
            provider: value
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            api_key: String::new(),
            api_key_set,
            base_url: value
                .get("baseUrl")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            model: value
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        }))
    }

    fn vault(&self) -> Result<std::sync::Arc<SecretVault>, WorkspaceError> {
        self.kernel
            .service::<SecretVault>()
            .map_err(|_| WorkspaceError::Message("secret vault not registered".into()))
    }

    pub fn record_generation_feedback(
        &self,
        project_id: &ProjectId,
        accepted: bool,
        ai_text: &str,
        human_text: &str,
        context_excerpt: &str,
    ) -> Result<Vec<PreferenceRule>, WorkspaceError> {
        let proposal_id = ProposalId::new();
        self.handle()?.execute(|repository| {
            if accepted {
                if let Some(record) = novel_feedback_memory::correction_from_edit(
                    proposal_id.clone(),
                    ai_text,
                    human_text,
                    context_excerpt,
                ) {
                    repository.save_correction(project_id, &record)?;
                }
            } else {
                let rule = novel_feedback_memory::rejection_rule(
                    RejectionReason::Other,
                    PreferenceScope::Project {
                        project_id: project_id.to_string(),
                    },
                    proposal_id,
                );
                repository.save_preference_rule(project_id, &rule)?;
            }
            Ok(())
        })?;
        self.list_preference_rules(project_id)
    }

    pub fn list_preference_rules(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<PreferenceRule>, WorkspaceError> {
        Ok(self
            .handle()?
            .execute(|repository| repository.list_preference_rules(project_id))?)
    }

    pub fn set_preference_status(
        &self,
        project_id: &ProjectId,
        rule_id: &PreferenceRuleId,
        disabled: bool,
    ) -> Result<Vec<PreferenceRule>, WorkspaceError> {
        let status = if disabled {
            PreferenceStatus::Disabled
        } else {
            PreferenceStatus::Confirmed
        };
        self.handle()?.execute(|repository| {
            repository.set_preference_status(project_id, rule_id, status)
        })?;
        self.list_preference_rules(project_id)
    }

    pub fn list_plugins(&self) -> Vec<PluginSummary> {
        novel_plugin_host::list_bundled_plugins()
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
            if config.api_key.is_empty() {
                config.api_key = self.secret_api_key().unwrap_or_default();
            }
            if !config.api_key.is_empty() {
                config.provider = resolve_provider_name(&config);
                return Ok(config);
            }
        }
        Ok(load_provider_config_from_kernel(self.kernel)?)
    }

    fn secret_api_key(&self) -> Result<String, WorkspaceError> {
        Ok(self.vault()?.get(MODEL_API_KEY)?.unwrap_or_default())
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
        let rules = if project_id.to_string().is_empty() {
            Vec::new()
        } else {
            self.list_preference_rules(&project_id).unwrap_or_default()
        };
        let spec = AgentSpec {
            id: Default::default(),
            project_id,
            chapter_id,
            base_revision: revision,
            prompt,
            context_text,
            budget: Default::default(),
            system_prompt: novel_feedback_memory::prompt_prefix(&rules),
            temperature: 0.8,
            emit_finish_event: true,
        };
        let report = self.kernel.run_continuation(&provider_config, spec).await?;
        Ok(report.patch)
    }

    pub fn propose_canon_from_chapter(
        &self,
        chapter_id: &ChapterId,
    ) -> Result<Vec<CanonProposal>, WorkspaceError> {
        let (project_id, created) = self.handle()?.execute(|repository| {
            let project_id = repository.chapter_project_id(chapter_id)?.ok_or_else(|| {
                StorageError::Domain(novel_domain::DomainError::NotFound(format!(
                    "chapter {chapter_id}"
                )))
            })?;
            let revision = repository.current_revision(chapter_id)?;
            let text = repository
                .chapter_text(chapter_id, revision)?
                .unwrap_or_default();
            let mentions = novel_story_model::extract_mentions(&text);
            let created =
                repository.propose_canon_mentions(&project_id, chapter_id, revision, &mentions)?;
            Ok((project_id, created))
        })?;
        if !created.is_empty() {
            self.dispatch_user(
                "canon.proposed",
                project_id,
                None,
                Some(chapter_id.clone()),
                json!({ "count": created.len() }),
            );
        }
        Ok(created)
    }

    pub fn list_canon(
        &self,
        project_id: &ProjectId,
        status: Option<FactStatus>,
    ) -> Result<Vec<CanonProposal>, WorkspaceError> {
        Ok(self
            .handle()?
            .execute(|repository| repository.list_canon_proposals(project_id, status))?)
    }

    pub fn review_canon_fact(
        &self,
        fact_id: &FactId,
        accept: bool,
    ) -> Result<CanonProposal, WorkspaceError> {
        let status = if accept {
            FactStatus::Accepted
        } else {
            FactStatus::Rejected
        };
        let proposal = self
            .handle()?
            .execute(|repository| repository.set_fact_status(fact_id, status))?;
        self.dispatch_user(
            if accept {
                "canon.accepted"
            } else {
                "canon.rejected"
            },
            proposal.project_id.clone(),
            None,
            proposal.chapter_id.clone(),
            json!({
                "factId": fact_id.to_string(),
                "entityName": proposal.entity_name,
            }),
        );
        Ok(proposal)
    }

    pub fn create_story_entry(
        &self,
        project_id: &ProjectId,
        kind: StoryEntryKind,
        title: &str,
        summary: &str,
    ) -> Result<StoryEntry, WorkspaceError> {
        let entry = self.handle()?.execute(|repository| {
            repository.create_story_entry(project_id, kind, title, summary)
        })?;
        self.dispatch_user(
            "story.entry.created",
            project_id.clone(),
            None,
            None,
            json!({
                "id": entry.id,
                "kind": serde_json::to_value(kind).unwrap_or(Value::Null),
                "title": title
            }),
        );
        Ok(entry)
    }

    pub fn list_story_entries(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<StoryEntry>, WorkspaceError> {
        Ok(self
            .handle()?
            .execute(|repository| repository.list_story_entries(project_id))?)
    }

    pub fn delete_story_entry(
        &self,
        project_id: &ProjectId,
        id: &str,
        kind: StoryEntryKind,
    ) -> Result<(), WorkspaceError> {
        self.handle()?
            .execute(|repository| repository.delete_story_entry(project_id, id, kind))?;
        self.dispatch_user(
            "story.entry.deleted",
            project_id.clone(),
            None,
            None,
            json!({
                "id": id,
                "kind": serde_json::to_value(kind).unwrap_or(Value::Null)
            }),
        );
        Ok(())
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
    if config.api_key.is_empty() {
        if let Ok(vault) = kernel.service::<SecretVault>() {
            if let Ok(Some(key)) = vault.get(MODEL_API_KEY) {
                config.api_key = key;
            }
        }
    } else if let Ok(vault) = kernel.service::<SecretVault>() {
        let leftover = config.api_key.clone();
        if vault.put(MODEL_API_KEY, &leftover).is_ok() {
            let stripped = json!({
                "provider": config.provider,
                "baseUrl": config.base_url,
                "model": config.model,
            });
            let _ = with_repository(kernel, |repository| {
                repository.save_setting("model_config", &stripped.to_string())
            });
        }
    }
    config.provider = resolve_provider_name(&config);
    Ok(config)
}
