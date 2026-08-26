use novel_automation::TypingSession;
use novel_domain::{
    Annotation, BlockId, Book, BookId, CanonProposal, Chapter, ChapterBody, ChapterId,
    ContentBlock, ContentPatch, DomainEvent, EventId, EventSource, FactId, FactStatus, JobView,
    LibrarySnapshot, Project, ProjectId, Revision, StoryEntry, StoryEntryKind, Volume, VolumeId,
    EVENT_SCHEMA_VERSION,
};
use novel_extensions::{BuiltinsExtension, SecretVault, Workspace};
use novel_kernel::{Kernel, ProviderConfig, ToolDescriptor};
use novel_storage::StorageHandle;
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

fn workspace(state: &AppState) -> Workspace<'_> {
    Workspace::new(&state.kernel)
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

    fn from_result(result: Result<T, impl ToString>) -> Self {
        match result {
            Ok(data) => Self::ok(data),
            Err(error) => Self::error(error),
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

fn notify_if_queued<R: tauri::Runtime>(app: &AppHandle<R>, queued: u64) {
    if queued > 0 {
        notify_queue_changed(app);
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
    #[serde(default)]
    pub volume_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewVolumeInput {
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
    #[serde(default)]
    pub lookback_text: String,
    pub generation: u64,
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

fn parse_volume_id(value: &str) -> Result<VolumeId, String> {
    value
        .parse()
        .map_err(|_| format!("invalid volume id: {value}"))
}

fn parse_chapter_id(value: &str) -> Result<ChapterId, String> {
    value
        .parse()
        .map_err(|_| format!("invalid chapter id: {value}"))
}

fn parse_fact_id(value: &str) -> Result<FactId, String> {
    value
        .parse()
        .map_err(|_| format!("invalid fact id: {value}"))
}

fn parse_fact_status(value: Option<&str>) -> Result<Option<FactStatus>, String> {
    match value {
        None | Some("") => Ok(None),
        Some("candidate") => Ok(Some(FactStatus::Candidate)),
        Some("accepted") => Ok(Some(FactStatus::Accepted)),
        Some("rejected") => Ok(Some(FactStatus::Rejected)),
        Some("superseded") => Ok(Some(FactStatus::Superseded)),
        Some(other) => Err(format!("invalid fact status: {other}")),
    }
}

fn parse_story_kind(value: &str) -> Result<StoryEntryKind, String> {
    match value {
        "character" => Ok(StoryEntryKind::Character),
        "setting" => Ok(StoryEntryKind::Setting),
        "foreshadow" => Ok(StoryEntryKind::Foreshadow),
        other => Err(format!("invalid story entry kind: {other}")),
    }
}

#[tauri::command]
fn create_project(state: State<'_, AppState>, input: NewProjectInput) -> CommandResult<Project> {
    info!(title = %input.title, "create_project 调用");
    CommandResult::from_result(workspace(&state).create_project(&input.title))
}

#[tauri::command]
fn create_book(state: State<'_, AppState>, input: NewBookInput) -> CommandResult<Book> {
    info!(project_id = %input.project_id, title = %input.title, "create_book 调用");
    let project_id = match parse_project_id(&input.project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).create_book(
        &project_id,
        &input.title,
        &input.synopsis,
        input.position,
    ))
}

#[tauri::command]
fn create_chapter(state: State<'_, AppState>, input: NewChapterInput) -> CommandResult<Chapter> {
    info!(project_id = %input.project_id, title = %input.title, "create_chapter 调用");
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
    CommandResult::from_result(workspace(&state).create_chapter(
        &project_id,
        &input.book_id,
        &input.title,
        input.position,
        input.volume_id.as_deref(),
    ))
}

#[tauri::command]
fn create_volume(state: State<'_, AppState>, input: NewVolumeInput) -> CommandResult<Volume> {
    let project_id = match parse_project_id(&input.project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    if parse_book_id(&input.book_id).is_err() {
        return CommandResult::error("invalid book id");
    }
    CommandResult::from_result(workspace(&state).create_volume(
        &project_id,
        &input.book_id,
        &input.title,
        input.position,
    ))
}

#[tauri::command]
fn load_library(
    state: State<'_, AppState>,
    project_id: Option<String>,
) -> CommandResult<LibrarySnapshot> {
    let project_id = match project_id {
        Some(id) => match parse_project_id(&id) {
            Ok(id) => Some(id),
            Err(err) => return CommandResult::error(err),
        },
        None => None,
    };
    CommandResult::from_result(workspace(&state).load_library(project_id))
}

#[tauri::command]
fn set_active_project(
    state: State<'_, AppState>,
    project_id: String,
) -> CommandResult<LibrarySnapshot> {
    let project_id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).set_active_project(project_id))
}

#[tauri::command]
fn load_chapter(state: State<'_, AppState>, chapter_id: String) -> CommandResult<ChapterBody> {
    let id = match parse_chapter_id(&chapter_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).load_chapter(&id))
}

#[tauri::command]
fn save_chapter(
    state: State<'_, AppState>,
    chapter_id: String,
    text: String,
    blocks: Option<Vec<ContentBlock>>,
) -> CommandResult<ChapterBody> {
    let id = match parse_chapter_id(&chapter_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).save_chapter(&id, &text, blocks))
}

#[tauri::command]
fn rename_project(
    state: State<'_, AppState>,
    project_id: String,
    title: String,
) -> CommandResult<LibrarySnapshot> {
    let id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).rename_project(&id, &title))
}

#[tauri::command]
fn delete_project(
    state: State<'_, AppState>,
    project_id: String,
) -> CommandResult<LibrarySnapshot> {
    let id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).delete_project(&id))
}

#[tauri::command]
fn rename_book(
    state: State<'_, AppState>,
    project_id: String,
    book_id: String,
    title: String,
) -> CommandResult<LibrarySnapshot> {
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let bid = match parse_book_id(&book_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).rename_book(&pid, &bid, &title))
}

#[tauri::command]
fn delete_book(
    state: State<'_, AppState>,
    project_id: String,
    book_id: String,
) -> CommandResult<LibrarySnapshot> {
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let bid = match parse_book_id(&book_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).delete_book(&pid, &bid))
}

#[tauri::command]
fn move_book(
    state: State<'_, AppState>,
    project_id: String,
    book_id: String,
    delta: i32,
) -> CommandResult<LibrarySnapshot> {
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let bid = match parse_book_id(&book_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).move_book(&pid, &bid, delta))
}

#[tauri::command]
fn rename_volume(
    state: State<'_, AppState>,
    project_id: String,
    volume_id: String,
    title: String,
) -> CommandResult<LibrarySnapshot> {
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let vid = match parse_volume_id(&volume_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).rename_volume(&pid, &vid, &title))
}

#[tauri::command]
fn delete_volume(
    state: State<'_, AppState>,
    project_id: String,
    volume_id: String,
) -> CommandResult<LibrarySnapshot> {
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let vid = match parse_volume_id(&volume_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).delete_volume(&pid, &vid))
}

#[tauri::command]
fn move_volume(
    state: State<'_, AppState>,
    project_id: String,
    volume_id: String,
    delta: i32,
) -> CommandResult<LibrarySnapshot> {
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let vid = match parse_volume_id(&volume_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).move_volume(&pid, &vid, delta))
}

#[tauri::command]
fn rename_chapter(
    state: State<'_, AppState>,
    project_id: String,
    chapter_id: String,
    title: String,
) -> CommandResult<LibrarySnapshot> {
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let cid = match parse_chapter_id(&chapter_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).rename_chapter(&pid, &cid, &title))
}

#[tauri::command]
fn delete_chapter(
    state: State<'_, AppState>,
    project_id: String,
    chapter_id: String,
) -> CommandResult<LibrarySnapshot> {
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let cid = match parse_chapter_id(&chapter_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).delete_chapter(&pid, &cid))
}

#[tauri::command]
fn move_chapter(
    state: State<'_, AppState>,
    project_id: String,
    chapter_id: String,
    delta: i32,
) -> CommandResult<LibrarySnapshot> {
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let cid = match parse_chapter_id(&chapter_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).move_chapter(&pid, &cid, delta))
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
    workspace(&state)
        .save_model_config(
            &config.provider,
            &config.api_key,
            &config.base_url,
            &config.model,
        )
        .map_err(|e| {
            error!(error = %e, "保存配置失败");
            e.to_string()
        })?;
    info!("模型配置已保存；API Key 不写入 SQLite");
    Ok(json!({ "saved": true }))
}

#[tauri::command]
fn load_model_config(state: State<'_, AppState>) -> Result<Value, String> {
    debug!("load_model_config 加载配置");
    match workspace(&state)
        .load_model_config()
        .map_err(|e| e.to_string())?
    {
        Some(config) => {
            info!(
                provider = %config.provider,
                model = %config.model,
                api_key_set = config.api_key_set,
                "加载模型配置（不含密钥明文）"
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
fn record_generation_feedback(
    state: State<'_, AppState>,
    project_id: String,
    accepted: bool,
    ai_text: String,
    human_text: Option<String>,
    context_excerpt: Option<String>,
) -> CommandResult<Vec<novel_domain::PreferenceRule>> {
    let project_id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).record_generation_feedback(
        &project_id,
        accepted,
        &ai_text,
        human_text.as_deref().unwrap_or(""),
        context_excerpt.as_deref().unwrap_or(""),
    ))
}

#[tauri::command]
fn list_preferences(
    state: State<'_, AppState>,
    project_id: String,
) -> CommandResult<Vec<novel_domain::PreferenceRule>> {
    let project_id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).list_preference_rules(&project_id))
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
    let override_config = config.map(|config| config.provider_config());
    let chapter = chapter_id.parse().unwrap_or_default();
    workspace(&state)
        .generate_continuation(
            chapter,
            Revision(revision),
            prompt,
            context_text,
            override_config,
        )
        .await
        .map_err(|e| e.to_string())
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
    match workspace(&state).save_annotation(&annotation) {
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

    let summary = workspace(&state).dispatch(&event);
    if let Some(err) = summary.first_error() {
        error!(error = err, "事件处理失败");
        return CommandResult::error(err);
    }

    let queued = summary.queued_count();
    info!(queued = queued, "领域事件处理完成");
    notify_if_queued(&app, queued);
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

    let mut event = DomainEvent::user(
        "block.mode.changed",
        project,
        None,
        Some(chapter),
        json!({
            "mode": mode,
            "previousMode": previous_mode,
            "blockId": block.as_ref().map(ToString::to_string),
            "position": position,
        }),
    );
    event.block_id = block;
    event.transaction_id = format!("mode:{}", EventId::new());
    event.schema_version = EVENT_SCHEMA_VERSION;
    event.source = EventSource::Editor;

    let summary = workspace(&state).dispatch(&event);
    if let Some(err) = summary.first_error() {
        error!(error = err, "模式切换事件处理失败");
        return CommandResult::error(err);
    }
    let queued = summary.queued_count();
    info!(queued = queued, "模式切换事件处理完成");
    notify_if_queued(&app, queued);
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
    let project_id = match input.project_id.parse() {
        Ok(id) => id,
        Err(_) => {
            warn!(input = %input.project_id, "enqueue_job 无效的 project_id");
            return CommandResult::error("invalid project id");
        }
    };
    match workspace(&state).enqueue_job(
        project_id,
        input.operation.clone(),
        input.payload,
        input.priority,
    ) {
        Ok((job_id, inserted)) => {
            info!(job_id = %job_id, inserted = inserted, "入队成功");
            notify_queue_changed(&app);
            CommandResult::ok(json!({
                "jobId": job_id.to_string(),
                "operation": input.operation,
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
fn list_jobs(state: State<'_, AppState>) -> CommandResult<Vec<JobView>> {
    CommandResult::from_result(workspace(&state).list_jobs(30))
}

#[tauri::command]
fn propose_canon(
    state: State<'_, AppState>,
    chapter_id: String,
) -> CommandResult<Vec<CanonProposal>> {
    let chapter_id = match parse_chapter_id(&chapter_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).propose_canon_from_chapter(&chapter_id))
}

#[tauri::command]
fn list_canon(
    state: State<'_, AppState>,
    project_id: String,
    status: Option<String>,
) -> CommandResult<Vec<CanonProposal>> {
    let project_id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let status = match parse_fact_status(status.as_deref()) {
        Ok(status) => status,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).list_canon(&project_id, status))
}

#[tauri::command]
fn review_canon_fact(
    state: State<'_, AppState>,
    fact_id: String,
    accept: bool,
) -> CommandResult<CanonProposal> {
    let fact_id = match parse_fact_id(&fact_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).review_canon_fact(&fact_id, accept))
}

#[tauri::command]
fn create_story_entry(
    state: State<'_, AppState>,
    project_id: String,
    kind: String,
    title: String,
    summary: String,
) -> CommandResult<StoryEntry> {
    let project_id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let kind = match parse_story_kind(&kind) {
        Ok(kind) => kind,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).create_story_entry(
        &project_id,
        kind,
        &title,
        &summary,
    ))
}

#[tauri::command]
fn list_story_entries(
    state: State<'_, AppState>,
    project_id: String,
) -> CommandResult<Vec<StoryEntry>> {
    let project_id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).list_story_entries(&project_id))
}

#[tauri::command]
fn delete_story_entry(
    state: State<'_, AppState>,
    project_id: String,
    id: String,
    kind: String,
) -> CommandResult<()> {
    let project_id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let kind = match parse_story_kind(&kind) {
        Ok(kind) => kind,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).delete_story_entry(&project_id, &id, kind))
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
            let storage = Arc::new(StorageHandle::open(database)?);
            let secrets = Arc::new(SecretVault::open(&data_dir));

            let kernel = Kernel::builder()
                .service(storage)
                .service(secrets)
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
            create_volume,
            rename_volume,
            delete_volume,
            move_volume,
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
            propose_canon,
            list_canon,
            review_canon_fact,
            create_story_entry,
            list_story_entries,
            delete_story_entry,
            record_generation_feedback,
            list_preferences,
        ])
        .run(tauri::generate_context!())
        .expect("error while running novel agent");
}
