use novel_automation::TypingSession;
use novel_domain::{
    Actor, Annotation, BlockId, BlockKind, Book, BookId, Chapter, ChapterId, ContentBlock,
    ContentPatch, DomainEvent, EventId, EventSource, Job, JobId, JobStatus, Platform, Project,
    ProjectId, Revision, EVENT_SCHEMA_VERSION,
};
use novel_extensions::BuiltinsExtension;
use novel_kernel::{AgentSpec, Kernel, ProviderConfig, ToolDescriptor};
use novel_storage::{Repository, SETTING_ACTIVE_PROJECT};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use tracing::{debug, error, info, warn};

pub struct AppState {
    pub kernel: Arc<Kernel>,
    pub typing_session: Mutex<TypingSession>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> CommandResult<T> {
    fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(error: impl ToString) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error.to_string()),
        }
    }
}

/// 通知前端队列有活动（入队或任务执行完成），由前端接管后续 drain。
/// 前端以此替代固定轮询：空闲时零 IPC、零日志。
fn notify_queue_changed<R: tauri::Runtime>(app: &AppHandle<R>) {
    let payload = json!({ "at": chrono::Utc::now().to_rfc3339() });
    if let Err(err) = app.emit("queue:changed", payload) {
        warn!(error = %err, "通知前端队列事件失败");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfigInput {
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

impl ModelConfigInput {
    /// 解析成内核 ProviderConfig：未知提供方按自定义 OpenAI 兼容处理。
    fn provider_config(&self) -> ProviderConfig {
        let mut config = ProviderConfig {
            provider: self.provider.clone(),
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
        };
        config.provider = novel_extensions::resolve_provider_name(&config);
        config
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewProjectInput {
    pub title: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewChapterInput {
    pub project_id: String,
    pub book_id: String,
    pub title: String,
    #[serde(default)]
    pub position: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewBookInput {
    pub project_id: String,
    pub title: String,
    #[serde(default)]
    pub synopsis: String,
    #[serde(default)]
    pub position: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    pub projects: Vec<Project>,
    pub active_project_id: Option<String>,
    pub books: Vec<Book>,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterBody {
    pub chapter_id: String,
    pub revision: u64,
    pub text: String,
    #[serde(default)]
    pub blocks: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorTickInput {
    #[allow(dead_code)]
    pub project_id: String,
    #[allow(dead_code)]
    pub chapter_id: String,
    pub revision: u64,
    pub chars_since_commit: u32,
    pub composing: bool,
    pub focused: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HintRequest {
    pub project_id: String,
    pub chapter_id: String,
    pub revision: u64,
    pub nearby_text: String,
    pub generation: u64,
}

fn repository(state: &AppState) -> Result<Arc<Mutex<Repository>>, String> {
    state
        .kernel
        .service::<Mutex<Repository>>()
        .map_err(|e| e.to_string())
}

fn parse_project_id(value: &str) -> Result<ProjectId, String> {
    value
        .parse()
        .map_err(|_| format!("invalid project id: {value}"))
}

fn parse_book_id(value: &str) -> Result<BookId, String> {
    value
        .parse()
        .map_err(|_| format!("invalid book id: {value}"))
}

fn parse_chapter_id(value: &str) -> Result<ChapterId, String> {
    value
        .parse()
        .map_err(|_| format!("invalid chapter id: {value}"))
}

fn user_event(
    event_type: &str,
    project_id: ProjectId,
    book_id: Option<BookId>,
    chapter_id: Option<ChapterId>,
    payload: Value,
) -> DomainEvent {
    DomainEvent {
        event_id: EventId::new(),
        event_type: event_type.into(),
        schema_version: EVENT_SCHEMA_VERSION,
        occurred_at: chrono::Utc::now(),
        project_id,
        book_id,
        chapter_id,
        scene_id: None,
        block_id: None,
        actor: Actor::User { user_id: None },
        source: EventSource::Editor,
        platform: Platform::Unknown,
        transaction_id: EventId::new().to_string(),
        correlation_id: None,
        causation_id: None,
        revision_before: Revision::INITIAL,
        revision_after: Revision::INITIAL,
        payload,
    }
}

#[tauri::command]
fn create_project(state: State<'_, AppState>, input: NewProjectInput) -> CommandResult<Project> {
    info!(title = %input.title, "create_project 调用");
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let project = {
        let guard = repository.lock().expect("repository poisoned");
        match guard.create_project(&input.title) {
            Ok(project) => {
                let _ = guard.save_setting(SETTING_ACTIVE_PROJECT, &project.id.to_string());
                project
            }
            Err(err) => {
                error!(error = %err, "create_project 失败");
                return CommandResult::error(err);
            }
        }
    };
    state.kernel.dispatch(&user_event(
        "project.created",
        project.id.clone(),
        None,
        None,
        json!({ "title": project.title }),
    ));
    info!(project_id = %project.id, "create_project 成功");
    CommandResult::ok(project)
}

#[tauri::command]
fn create_book(state: State<'_, AppState>, input: NewBookInput) -> CommandResult<Book> {
    info!(project_id = %input.project_id, title = %input.title, "create_book 调用");
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let project_id = match parse_project_id(&input.project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let book = {
        let guard = repository.lock().expect("repository poisoned");
        match guard.create_book(&project_id, &input.title, &input.synopsis, input.position) {
            Ok(book) => book,
            Err(err) => {
                error!(error = %err, "create_book 失败");
                return CommandResult::error(err);
            }
        }
    };
    state.kernel.dispatch(&user_event(
        "book.created",
        project_id,
        Some(book.id.clone()),
        None,
        json!({ "title": book.title, "position": book.position }),
    ));
    info!(book_id = %book.id, "create_book 成功");
    CommandResult::ok(book)
}

#[tauri::command]
fn create_chapter(state: State<'_, AppState>, input: NewChapterInput) -> CommandResult<Chapter> {
    info!(project_id = %input.project_id, title = %input.title, "create_chapter 调用");
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let project_id = match parse_project_id(&input.project_id) {
        Ok(id) => id,
        Err(_) => {
            warn!(input = %input.project_id, "create_chapter 无效的 project_id");
            return CommandResult::error("invalid project id");
        }
    };
    if parse_book_id(&input.book_id).is_err() {
        return CommandResult::error("invalid book id");
    }
    let chapter = {
        let guard = repository.lock().expect("repository poisoned");
        match guard.create_chapter(&project_id, &input.book_id, &input.title, input.position) {
            Ok(chapter) => chapter,
            Err(err) => {
                error!(error = %err, "create_chapter 失败");
                return CommandResult::error(err);
            }
        }
    };
    state.kernel.dispatch(&user_event(
        "chapter.created",
        project_id,
        Some(chapter.book_id.clone()),
        Some(chapter.id.clone()),
        json!({
            "title": chapter.title,
            "bookId": chapter.book_id.to_string(),
            "position": chapter.position,
        }),
    ));
    info!(chapter_id = %chapter.id, "create_chapter 成功");
    CommandResult::ok(chapter)
}

#[tauri::command]
fn load_library(
    state: State<'_, AppState>,
    project_id: Option<String>,
) -> CommandResult<LibrarySnapshot> {
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let guard = repository.lock().expect("repository poisoned");
    let projects = match guard.list_projects() {
        Ok(value) => value,
        Err(err) => return CommandResult::error(err),
    };
    let stored = guard.get_setting(SETTING_ACTIVE_PROJECT).ok().flatten();
    let active = project_id
        .or(stored)
        .or_else(|| projects.first().map(|project| project.id.to_string()));
    if let Some(id) = &active {
        let _ = guard.save_setting(SETTING_ACTIVE_PROJECT, id);
    }
    let (books, chapters) = if let Some(id) = &active {
        match parse_project_id(id) {
            Ok(pid) => match (guard.list_books(&pid), guard.list_chapters(&pid)) {
                (Ok(books), Ok(chapters)) => (books, chapters),
                (Err(err), _) | (_, Err(err)) => return CommandResult::error(err),
            },
            Err(err) => return CommandResult::error(err),
        }
    } else {
        (Vec::new(), Vec::new())
    };
    CommandResult::ok(LibrarySnapshot {
        projects,
        active_project_id: active,
        books,
        chapters,
    })
}

#[tauri::command]
fn set_active_project(
    state: State<'_, AppState>,
    project_id: String,
) -> CommandResult<LibrarySnapshot> {
    load_library(state, Some(project_id))
}

fn read_chapter_body(guard: &Repository, id: &ChapterId) -> Result<ChapterBody, String> {
    let revision = guard.current_revision(id).map_err(|err| err.to_string())?;
    let text = guard
        .chapter_text(id, revision)
        .map_err(|err| err.to_string())?
        .unwrap_or_default();
    let blocks = match guard
        .block_sequence(id, revision)
        .map_err(|err| err.to_string())?
    {
        Some(sequence) => sequence.blocks,
        None if text.is_empty() => Vec::new(),
        None => vec![ContentBlock {
            id: BlockId::new(),
            kind: BlockKind::Body,
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

#[tauri::command]
fn load_chapter(state: State<'_, AppState>, chapter_id: String) -> CommandResult<ChapterBody> {
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let id = match parse_chapter_id(&chapter_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let guard = repository.lock().expect("repository poisoned");
    match read_chapter_body(&guard, &id) {
        Ok(body) => CommandResult::ok(body),
        Err(err) => CommandResult::error(err),
    }
}

#[tauri::command]
fn save_chapter(
    state: State<'_, AppState>,
    chapter_id: String,
    text: String,
    blocks: Option<Vec<ContentBlock>>,
) -> CommandResult<ChapterBody> {
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let id = match parse_chapter_id(&chapter_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let mut guard = repository.lock().expect("repository poisoned");
    let saved = if let Some(blocks) = blocks {
        guard.save_block_sequence(&id, &blocks)
    } else {
        guard.save_chapter_snapshot(&id, &text, "user")
    };
    match saved {
        Ok(_) => match read_chapter_body(&guard, &id) {
            Ok(body) => {
                info!(chapter_id = %chapter_id, revision = body.revision, "save_chapter 成功");
                CommandResult::ok(body)
            }
            Err(err) => CommandResult::error(err),
        },
        Err(err) => {
            error!(error = %err, "save_chapter 失败");
            CommandResult::error(err)
        }
    }
}

#[tauri::command]
fn rename_project(
    state: State<'_, AppState>,
    project_id: String,
    title: String,
) -> CommandResult<LibrarySnapshot> {
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    {
        let guard = repository.lock().expect("repository poisoned");
        if let Err(err) = guard.rename_project(&id, &title) {
            return CommandResult::error(err);
        }
    }
    state.kernel.dispatch(&user_event(
        "project.renamed",
        id,
        None,
        None,
        json!({ "title": title }),
    ));
    load_library(state, Some(project_id))
}

#[tauri::command]
fn delete_project(
    state: State<'_, AppState>,
    project_id: String,
) -> CommandResult<LibrarySnapshot> {
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    {
        let guard = repository.lock().expect("repository poisoned");
        if let Err(err) = guard.delete_project(&id) {
            return CommandResult::error(err);
        }
    }
    state
        .kernel
        .dispatch(&user_event("project.deleted", id, None, None, json!({})));
    load_library(state, None)
}

#[tauri::command]
fn rename_book(
    state: State<'_, AppState>,
    project_id: String,
    book_id: String,
    title: String,
) -> CommandResult<LibrarySnapshot> {
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let bid = match parse_book_id(&book_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    {
        let guard = repository.lock().expect("repository poisoned");
        if let Err(err) = guard.rename_book(&pid, &bid, &title) {
            return CommandResult::error(err);
        }
    }
    state.kernel.dispatch(&user_event(
        "book.renamed",
        pid,
        Some(bid),
        None,
        json!({ "title": title }),
    ));
    load_library(state, Some(project_id))
}

#[tauri::command]
fn delete_book(
    state: State<'_, AppState>,
    project_id: String,
    book_id: String,
) -> CommandResult<LibrarySnapshot> {
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let bid = match parse_book_id(&book_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    {
        let guard = repository.lock().expect("repository poisoned");
        if let Err(err) = guard.delete_book(&pid, &bid) {
            return CommandResult::error(err);
        }
    }
    state
        .kernel
        .dispatch(&user_event("book.deleted", pid, Some(bid), None, json!({})));
    load_library(state, Some(project_id))
}

#[tauri::command]
fn move_book(
    state: State<'_, AppState>,
    project_id: String,
    book_id: String,
    delta: i32,
) -> CommandResult<LibrarySnapshot> {
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let bid = match parse_book_id(&book_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    {
        let guard = repository.lock().expect("repository poisoned");
        if let Err(err) = guard.move_book(&pid, &bid, delta) {
            return CommandResult::error(err);
        }
    }
    state.kernel.dispatch(&user_event(
        "book.reordered",
        pid,
        Some(bid),
        None,
        json!({ "delta": delta }),
    ));
    load_library(state, Some(project_id))
}

#[tauri::command]
fn rename_chapter(
    state: State<'_, AppState>,
    project_id: String,
    chapter_id: String,
    title: String,
) -> CommandResult<LibrarySnapshot> {
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let cid = match parse_chapter_id(&chapter_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    {
        let guard = repository.lock().expect("repository poisoned");
        if let Err(err) = guard.rename_chapter(&pid, &cid, &title) {
            return CommandResult::error(err);
        }
    }
    state.kernel.dispatch(&user_event(
        "chapter.renamed",
        pid,
        None,
        Some(cid),
        json!({ "title": title }),
    ));
    load_library(state, Some(project_id))
}

#[tauri::command]
fn delete_chapter(
    state: State<'_, AppState>,
    project_id: String,
    chapter_id: String,
) -> CommandResult<LibrarySnapshot> {
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let cid = match parse_chapter_id(&chapter_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    {
        let guard = repository.lock().expect("repository poisoned");
        if let Err(err) = guard.delete_chapter(&pid, &cid) {
            return CommandResult::error(err);
        }
    }
    state.kernel.dispatch(&user_event(
        "chapter.deleted",
        pid,
        None,
        Some(cid),
        json!({}),
    ));
    load_library(state, Some(project_id))
}

#[tauri::command]
fn move_chapter(
    state: State<'_, AppState>,
    project_id: String,
    chapter_id: String,
    delta: i32,
) -> CommandResult<LibrarySnapshot> {
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let cid = match parse_chapter_id(&chapter_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    {
        let guard = repository.lock().expect("repository poisoned");
        if let Err(err) = guard.move_chapter(&pid, &cid, delta) {
            return CommandResult::error(err);
        }
    }
    state.kernel.dispatch(&user_event(
        "chapter.reordered",
        pid,
        None,
        Some(cid),
        json!({ "delta": delta }),
    ));
    load_library(state, Some(project_id))
}

#[tauri::command]
fn editor_tick(state: State<'_, AppState>, input: EditorTickInput) -> CommandResult<Value> {
    debug!(
        revision = input.revision,
        chars = input.chars_since_commit,
        composing = input.composing,
        "editor_tick"
    );
    let mut typing = state.typing_session.lock().expect("typing poisoned");
    typing.composing = input.composing;
    typing.focused = input.focused;
    typing.chars_since_commit = input.chars_since_commit;
    typing.last_input_at = chrono::Utc::now();

    let should_idle =
        typing.should_emit_idle(chrono::Utc::now(), chrono::Duration::milliseconds(1800), 20);

    if should_idle {
        info!(revision = input.revision, "检测到停笔，触发 idle 事件");
    }

    CommandResult::ok(json!({
        "shouldEmitIdle": should_idle,
        "revision": input.revision,
    }))
}

#[tauri::command]
async fn context_hints(
    state: State<'_, AppState>,
    input: HintRequest,
) -> Result<CommandResult<Value>, String> {
    debug!(
        revision = input.revision,
        text_len = input.nearby_text.len(),
        "context_hints 查询"
    );
    match state.kernel.call_tool("context.hints", json!(input)).await {
        Ok(hints) => {
            let count = hints.as_array().map(Vec::len).unwrap_or(0);
            info!(count = count, "context_hints 返回结果");
            Ok(CommandResult::ok(hints))
        }
        Err(err) => {
            error!(error = %err, "context_hints 失败");
            Ok(CommandResult::error(err))
        }
    }
}

#[tauri::command]
async fn save_model_config(
    state: State<'_, AppState>,
    config: ModelConfigInput,
) -> Result<Value, String> {
    info!(
        provider = %config.provider,
        model = %config.model,
        base_url = %config.base_url,
        "save_model_config 保存配置"
    );

    let repository = repository(&state)?;
    let json_value = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    let guard = repository.lock().map_err(|e| e.to_string())?;
    guard
        .save_setting("model_config", &json_value)
        .map_err(|e| {
            error!(error = %e, "保存配置到数据库失败");
            e.to_string()
        })?;

    info!("模型配置已保存到数据库");
    Ok(json!({ "saved": true }))
}

#[tauri::command]
fn load_model_config(state: State<'_, AppState>) -> Result<Value, String> {
    debug!("load_model_config 加载配置");

    let repository = repository(&state)?;
    let guard = repository.lock().map_err(|e| e.to_string())?;
    let json_value = guard
        .get_setting("model_config")
        .map_err(|e| e.to_string())?;

    match json_value {
        Some(json_str) => {
            let config: ModelConfigInput =
                serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
            info!(
                provider = %config.provider,
                model = %config.model,
                "从数据库加载模型配置"
            );
            Ok(json!(config))
        }
        None => {
            debug!("数据库中无保存的模型配置");
            Ok(json!(null))
        }
    }
}

#[tauri::command]
async fn generate_continuation(
    state: State<'_, AppState>,
    chapter_id: String,
    revision: u64,
    prompt: String,
    context_text: String,
    config: Option<ModelConfigInput>,
) -> Result<ContentPatch, String> {
    info!(
        chapter_id = %chapter_id,
        revision = revision,
        prompt_len = prompt.len(),
        "generate_continuation 调用"
    );

    // 优先使用传入的配置，否则读取已保存的配置，最后回退 echo。
    let config = match config {
        Some(config) => Some(config),
        None => load_saved_model_config(&state)?,
    };
    let provider_config = match &config {
        Some(config) if !config.api_key.is_empty() => {
            info!(
                provider = %config.provider,
                model = %config.model,
                base_url = %config.base_url,
                "使用 OpenAI 兼容 Provider"
            );
            config.provider_config()
        }
        _ => {
            warn!("未配置模型，使用 EchoProvider 回退");
            ProviderConfig {
                provider: "echo".into(),
                ..Default::default()
            }
        }
    };

    let chapter = chapter_id.parse().unwrap_or_default();
    let project_id = repository(&state)
        .ok()
        .and_then(|repository| {
            repository
                .lock()
                .ok()
                .and_then(|guard| guard.chapter_project_id(&chapter).ok().flatten())
        })
        .unwrap_or_default();

    let spec = AgentSpec {
        id: Default::default(),
        project_id,
        chapter_id: chapter,
        base_revision: novel_domain::Revision(revision),
        prompt,
        context_text,
        budget: Default::default(),
        system_prompt: None,
        temperature: 0.8,
        emit_finish_event: true,
    };

    info!("开始调用模型生成续写...");
    let result = state.kernel.run_continuation(&provider_config, spec).await;

    match &result {
        Ok(report) => info!(
            operations = report.patch.operations.len(),
            truncated = report.truncated,
            elapsed_ms = report.elapsed_ms,
            "generate_continuation 成功"
        ),
        Err(err) => error!(error = %err, "generate_continuation 失败"),
    }

    result.map(|report| report.patch).map_err(|e| e.to_string())
}

fn load_saved_model_config(state: &AppState) -> Result<Option<ModelConfigInput>, String> {
    let repository = repository(state)?;
    let guard = repository.lock().map_err(|e| e.to_string())?;
    match guard
        .get_setting("model_config")
        .map_err(|e| e.to_string())?
    {
        Some(json_str) => Ok(Some(
            serde_json::from_str(&json_str).map_err(|e| e.to_string())?,
        )),
        None => Ok(None),
    }
}

#[tauri::command]
async fn install_plugin_manifest(
    state: State<'_, AppState>,
    manifest_json: String,
) -> Result<CommandResult<Value>, String> {
    info!(
        json_len = manifest_json.len(),
        "install_plugin_manifest 调用"
    );
    match state
        .kernel
        .call_tool("plugin.install", json!({ "manifestJson": manifest_json }))
        .await
    {
        Ok(value) => {
            info!("插件清单解析成功");
            Ok(CommandResult::ok(value))
        }
        Err(err) => {
            error!(error = %err, "install_plugin_manifest 解析失败");
            Ok(CommandResult::error(err))
        }
    }
}

#[tauri::command]
fn commit_annotation(state: State<'_, AppState>, annotation: Annotation) -> CommandResult<Value> {
    info!(annotation_id = %annotation.id, "commit_annotation 保存批注");
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let guard = repository.lock().expect("repository poisoned");
    match guard.save_annotation(&annotation) {
        Ok(()) => {
            info!("批注保存成功");
            CommandResult::ok(json!({ "saved": true }))
        }
        Err(err) => {
            error!(error = %err, "批注保存失败");
            CommandResult::error(err)
        }
    }
}

#[tauri::command]
fn emit_domain_event<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    event: DomainEvent,
) -> CommandResult<Value> {
    info!(
        event_type = %event.event_type,
        event_id = %event.event_id,
        project_id = %event.project_id,
        "emit_domain_event 触发领域事件"
    );

    let summary = state.kernel.dispatch(&event);
    if let Some(err) = summary.first_error() {
        error!(error = err, "事件处理失败");
        return CommandResult::error(err);
    }

    let queued: u64 = summary
        .outcomes
        .iter()
        .filter_map(|outcome| {
            outcome
                .output
                .as_ref()
                .and_then(|output| output.get("queued"))
                .and_then(Value::as_u64)
        })
        .sum();
    info!(queued = queued, "领域事件处理完成");
    if queued > 0 {
        notify_queue_changed(&app);
    }
    CommandResult::ok(json!({ "recorded": true, "queued": queued }))
}

/// 思考/正文模式切换信号量：
/// 前端在"新行行首"状态按 Tab 切换块类型时调用，构造领域事件
/// `block.mode.changed` 并 dispatch——工作流规则可监听该事件，
/// 匹配后自动入队对应的任务序列（插件扩展点）。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn emit_block_mode_changed<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    project_id: String,
    chapter_id: String,
    mode: String,
    previous_mode: String,
    block_id: Option<String>,
    position: Option<u32>,
) -> CommandResult<Value> {
    info!(
        mode = %mode,
        previous_mode = %previous_mode,
        project_id = %project_id,
        "emit_block_mode_changed 块模式切换"
    );
    let Ok(project) = ProjectId::from_str(&project_id) else {
        return CommandResult::error(format!("invalid project id: {project_id}"));
    };
    let Ok(chapter) = ChapterId::from_str(&chapter_id) else {
        return CommandResult::error(format!("invalid chapter id: {chapter_id}"));
    };
    let block = block_id.and_then(|id| BlockId::from_str(&id).ok());

    let event = DomainEvent {
        event_id: EventId::new(),
        event_type: "block.mode.changed".into(),
        schema_version: EVENT_SCHEMA_VERSION,
        occurred_at: chrono::Utc::now(),
        project_id: project,
        book_id: None,
        chapter_id: Some(chapter),
        scene_id: None,
        block_id: block.clone(),
        actor: Actor::User { user_id: None },
        source: EventSource::Editor,
        platform: Platform::Unknown,
        transaction_id: format!("mode:{}", EventId::new()),
        correlation_id: None,
        causation_id: None,
        revision_before: Revision::INITIAL,
        revision_after: Revision::INITIAL,
        payload: json!({
            "mode": mode,
            "previousMode": previous_mode,
            "blockId": block.map(|b| b.to_string()),
            "position": position,
        }),
    };

    let summary = state.kernel.dispatch(&event);
    if let Some(err) = summary.first_error() {
        error!(error = err, "模式切换事件处理失败");
        return CommandResult::error(err);
    }
    let queued: u64 = summary
        .outcomes
        .iter()
        .filter_map(|outcome| {
            outcome
                .output
                .as_ref()
                .and_then(|output| output.get("queued"))
                .and_then(Value::as_u64)
        })
        .sum();
    info!(queued = queued, "模式切换事件处理完成");
    if queued > 0 {
        notify_queue_changed(&app);
    }
    CommandResult::ok(json!({ "recorded": true, "queued": queued }))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn build_context_package(
    state: State<'_, AppState>,
    project_id: String,
    chapter_id: String,
    revision: u64,
    instruction: String,
    current_scene: String,
    pinned: Vec<String>,
    retrieved: Vec<String>,
    summaries: Vec<String>,
) -> Result<CommandResult<Value>, String> {
    info!(
        project_id = %project_id,
        chapter_id = %chapter_id,
        revision = revision,
        "build_context_package 组装上下文包"
    );
    let input = json!({
        "projectId": project_id,
        "chapterId": chapter_id,
        "revision": revision,
        "instruction": instruction,
        "currentScene": current_scene,
        "pinned": pinned,
        "retrieved": retrieved,
        "summaries": summaries,
    });
    match state.kernel.call_tool("context.assemble", input).await {
        Ok(package) => {
            let sections = package
                .get("sections")
                .and_then(|value| value.as_array())
                .map(Vec::len)
                .unwrap_or(0);
            info!(sections = sections, "上下文包组装完成");
            Ok(CommandResult::ok(package))
        }
        Err(err) => {
            error!(error = %err, "上下文包组装失败");
            Ok(CommandResult::error(err))
        }
    }
}

#[tauri::command]
async fn run_queue_step<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<CommandResult<Value>, String> {
    match state.kernel.call_tool("queue.tick", json!({})).await {
        Ok(result) => {
            if result
                .get("executed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                info!(
                    job_id = result.get("jobId").and_then(|v| v.as_str()).unwrap_or(""),
                    operation = result
                        .get("operation")
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                    success = result
                        .get("success")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    "队列任务执行完成"
                );
                // 执行了一个任务：通知前端继续 drain（可能还有后续任务）
                notify_queue_changed(&app);
            }
            Ok(CommandResult::ok(result))
        }
        Err(err) => {
            error!(error = %err, "队列任务执行失败");
            Ok(CommandResult::error(err))
        }
    }
}

/// 内核自描述：已注册的工具与提供方，便于前端/调试发现扩展能力。
#[tauri::command]
fn kernel_tools(state: State<'_, AppState>) -> CommandResult<Value> {
    let tools: Vec<ToolDescriptor> = state.kernel.tool_registry().describe();
    let providers = state.kernel.provider_registry().names();
    CommandResult::ok(json!({ "tools": tools, "providers": providers }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueJobInput {
    pub project_id: String,
    pub operation: String,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub priority: i32,
}

/// 手动入队一个任务。队列由前端事件驱动执行（`queue:changed` → drain）。
#[tauri::command]
fn enqueue_job<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    input: EnqueueJobInput,
) -> CommandResult<Value> {
    info!(operation = %input.operation, "enqueue_job 手动入队");
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let guard = repository.lock().expect("repository poisoned");
    let project_id = match input.project_id.parse() {
        Ok(id) => id,
        Err(_) => {
            warn!(input = %input.project_id, "enqueue_job 无效的 project_id");
            return CommandResult::error("invalid project id");
        }
    };
    let now = chrono::Utc::now();
    let job = Job {
        id: JobId::new(),
        project_id,
        workflow_id: None,
        operation: input.operation.clone(),
        payload: input.payload,
        priority: input.priority,
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
    match guard.enqueue_job(&job) {
        Ok(inserted) => {
            info!(job_id = %job.id, inserted = inserted, "入队成功");
            notify_queue_changed(&app);
            CommandResult::ok(json!({
                "jobId": job.id.to_string(),
                "operation": job.operation,
                "inserted": inserted,
            }))
        }
        Err(err) => {
            error!(error = %err, "入队失败");
            CommandResult::error(err)
        }
    }
}

/// 查询最近队列任务（含状态），供前端刷新任务面板。
#[tauri::command]
fn list_jobs(state: State<'_, AppState>) -> CommandResult<Vec<Value>> {
    let Ok(repository) = repository(&state) else {
        return CommandResult::error("repository unavailable");
    };
    let guard = repository.lock().expect("repository poisoned");
    match guard.list_jobs(30) {
        Ok(jobs) => {
            let values: Vec<Value> = jobs
                .iter()
                .map(|job| {
                    json!({
                        "id": job.id.to_string(),
                        "operation": job.operation,
                        "status": serde_json::to_value(job.status).unwrap_or(Value::Null),
                        "attempts": job.attempts,
                        "createdAt": job.created_at.to_rfc3339(),
                        "updatedAt": job.updated_at.to_rfc3339(),
                    })
                })
                .collect();
            CommandResult::ok(values)
        }
        Err(err) => {
            error!(error = %err, "查询任务列表失败");
            CommandResult::error(err)
        }
    }
}

#[cfg(test)]
mod command_tests;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into()),
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let database = data_dir.join("novel-agent.sqlite3");
            let repository = Arc::new(Mutex::new(Repository::open(database)?));

            let kernel = Kernel::builder()
                .service(repository)
                .extension(BuiltinsExtension)?
                .build()?;
            app.manage(AppState {
                kernel: Arc::new(kernel),
                typing_session: Mutex::new(TypingSession::new(chrono::Utc::now())),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_project,
            create_book,
            create_chapter,
            load_library,
            set_active_project,
            load_chapter,
            save_chapter,
            rename_project,
            delete_project,
            rename_book,
            delete_book,
            move_book,
            rename_chapter,
            delete_chapter,
            move_chapter,
            editor_tick,
            context_hints,
            save_model_config,
            load_model_config,
            generate_continuation,
            install_plugin_manifest,
            commit_annotation,
            emit_domain_event,
            emit_block_mode_changed,
            build_context_package,
            run_queue_step,
            kernel_tools,
            enqueue_job,
            list_jobs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running novel agent");
}
