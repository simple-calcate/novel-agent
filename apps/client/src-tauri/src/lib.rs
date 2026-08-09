use novel_agent_runtime::{AgentBudget, AgentRun, AgentRuntime, EchoProvider, OpenAICompatibleProvider};
use novel_automation::{rule_matches, JobQueue, JobRunner, TypingSession};
use novel_context_engine::{assemble_context, AssemblyOptions};
use novel_context_hints::{HintEngine, HintQuery};
use novel_domain::{
    Annotation, ChapterId, ContentPatch, DomainEvent, ProjectId, Revision, WorkContextRef,
};
use novel_plugin_host::{evaluate, parse_manifest};
use novel_storage::Repository;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Mutex;
use tauri::{Manager, State};
use tracing::{debug, error, info, warn};

pub struct AppState {
    pub repository: Mutex<Repository>,
    pub typing_session: Mutex<TypingSession>,
    pub model_config: Mutex<Option<ModelConfigInput>>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfigInput {
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
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
    pub position: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorTickInput {
    pub project_id: String,
    pub chapter_id: String,
    pub revision: u64,
    pub chars_since_commit: u32,
    pub composing: bool,
    pub focused: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HintRequest {
    pub project_id: String,
    pub chapter_id: String,
    pub revision: u64,
    pub nearby_text: String,
    pub generation: u64,
}

#[tauri::command]
fn create_project(
    state: State<'_, AppState>,
    input: NewProjectInput,
) -> CommandResult<novel_domain::Project> {
    info!(title = %input.title, "create_project 调用");
    let repository = state.repository.lock().expect("repository poisoned");
    match repository.create_project(&input.title) {
        Ok(project) => {
            info!(project_id = %project.id, "create_project 成功");
            CommandResult::ok(project)
        }
        Err(error) => {
            error!(error = %error, "create_project 失败");
            CommandResult::error(error)
        }
    }
}

#[tauri::command]
fn create_chapter(
    state: State<'_, AppState>,
    input: NewChapterInput,
) -> CommandResult<novel_domain::Chapter> {
    info!(project_id = %input.project_id, title = %input.title, "create_chapter 调用");
    let repository = state.repository.lock().expect("repository poisoned");
    let project_id = match input.project_id.parse() {
        Ok(id) => id,
        Err(_) => {
            warn!(input = %input.project_id, "create_chapter 无效的 project_id");
            return CommandResult::error("invalid project id");
        }
    };
    match repository.create_chapter(&project_id, &input.book_id, &input.title, input.position) {
        Ok(chapter) => {
            info!(chapter_id = %chapter.id, "create_chapter 成功");
            CommandResult::ok(chapter)
        }
        Err(error) => {
            error!(error = %error, "create_chapter 失败");
            CommandResult::error(error)
        }
    }
}

#[tauri::command]
fn editor_tick(
    state: State<'_, AppState>,
    input: EditorTickInput,
) -> CommandResult<Value> {
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

    let should_idle = typing.should_emit_idle(
        chrono::Utc::now(),
        chrono::Duration::milliseconds(1800),
        20,
    );

    if should_idle {
        info!(revision = input.revision, "检测到停笔，触发 idle 事件");
    }

    CommandResult::ok(json!({
        "shouldEmitIdle": should_idle,
        "revision": input.revision,
    }))
}

#[tauri::command]
fn context_hints(state: State<'_, AppState>, input: HintRequest) -> CommandResult<Value> {
    debug!(
        revision = input.revision,
        text_len = input.nearby_text.len(),
        "context_hints 查询"
    );
    let repository = state.repository.lock().expect("repository poisoned");
    let _ = &repository;
    let query = HintQuery {
        work_ref: WorkContextRef {
            project_id: input.project_id.parse().unwrap_or_else(|_| ProjectId::new()),
            branch_id: "main".into(),
            revision: Revision(input.revision),
            chapter_id: input.chapter_id.parse().unwrap_or_else(|_| ChapterId::new()),
            block_id: None,
            pov_entity_id: None,
        },
        nearby_text: input.nearby_text,
        generation: input.generation,
        limit: 5,
    };
    let hints = HintEngine {
        minimum_dwell_score: 0.2,
    }
    .rank(&query, &[], &[], &[]);
    info!(count = hints.len(), "context_hints 返回结果");
    CommandResult::ok(json!(hints))
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

    // 保存到内存
    {
        let mut model_config = state.model_config.lock().map_err(|e| {
            error!(error = %e, "获取 model_config 锁失败");
            e.to_string()
        })?;
        *model_config = Some(config.clone());
    }

    // 持久化到 SQLite
    let repository = state.repository.lock().map_err(|e| e.to_string())?;
    let json_value = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    repository
        .save_setting("model_config", &json_value)
        .map_err(|e| {
            error!(error = %e, "保存配置到数据库失败");
            e.to_string()
        })?;

    info!("模型配置已保存到内存和数据库");
    Ok(json!({ "saved": true }))
}

#[tauri::command]
fn load_model_config(state: State<'_, AppState>) -> Result<Value, String> {
    debug!("load_model_config 加载配置");

    let repository = state.repository.lock().map_err(|e| e.to_string())?;
    let json_value = repository
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

            // 同时更新内存
            let mut model_config = state.model_config.lock().map_err(|e| e.to_string())?;
            *model_config = Some(config.clone());

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

    // 优先使用传入的配置，否则用已保存的配置
    let config = config.or_else(|| {
        let saved = state.model_config.lock().expect("model config poisoned");
        saved.clone()
    });

    let provider: Box<dyn novel_agent_runtime::ModelProvider> = match &config {
        Some(c) if !c.api_key.is_empty() => {
            info!(
                provider = %c.provider,
                model = %c.model,
                base_url = %c.base_url,
                "使用 OpenAI 兼容 Provider"
            );
            Box::new(OpenAICompatibleProvider {
                base_url: c.base_url.clone(),
                api_key: c.api_key.clone(),
                provider_name: c.provider.clone(),
            })
        }
        _ => {
            warn!("未配置模型，使用 EchoProvider 回退");
            Box::new(EchoProvider)
        }
    };

    let model_name = config.as_ref().map(|c| c.model.clone()).unwrap_or_else(|| "default".into());
    let runtime = AgentRuntime::new_provider(provider).with_model(model_name);
    let run = AgentRun {
        id: Default::default(),
        chapter_id: chapter_id.parse().unwrap_or_else(|_| ChapterId::new()),
        base_revision: Revision(revision),
        prompt: prompt.clone(),
        budget: AgentBudget {
            max_rounds: 2,
            max_tokens: 2048,
            max_cost_micros: 100_000,
            max_seconds: 120,
        },
        started_at: chrono::Utc::now(),
    };

    info!("开始调用模型生成续写...");
    let result = runtime.run_continuation(run, context_text).await;

    match &result {
        Ok(patch) => info!(
            operations = patch.operations.len(),
            "generate_continuation 成功"
        ),
        Err(e) => error!(error = %e, "generate_continuation 失败"),
    }

    result.map_err(|e| e.to_string())
}

#[tauri::command]
fn install_plugin_manifest(manifest_json: String) -> CommandResult<Value> {
    info!(json_len = manifest_json.len(), "install_plugin_manifest 调用");
    let platform = if cfg!(target_os = "android") {
        novel_domain::PluginPlatform::Android
    } else if cfg!(target_os = "windows") {
        novel_domain::PluginPlatform::Windows
    } else {
        novel_domain::PluginPlatform::Linux
    };

    match parse_manifest(&manifest_json, platform) {
        Ok(manifest) => {
            info!(plugin_id = %manifest.id, name = %manifest.name, "插件清单解析成功");
            let decision = evaluate(&manifest, &manifest.requested_capabilities);
            info!(
                granted = decision.granted.len(),
                denied = decision.denied.len(),
                "权限评估完成"
            );
            CommandResult::ok(json!({
                "manifest": manifest,
                "granted": decision.granted,
                "denied": decision.denied,
            }))
        }
        Err(error) => {
            error!(error = %error, "install_plugin_manifest 解析失败");
            CommandResult::error(error)
        }
    }
}

#[tauri::command]
fn commit_annotation(
    state: State<'_, AppState>,
    annotation: Annotation,
) -> CommandResult<Value> {
    info!(annotation_id = %annotation.id, "commit_annotation 保存批注");
    let repository = state.repository.lock().expect("repository poisoned");
    match repository.save_annotation(&annotation) {
        Ok(()) => {
            info!("批注保存成功");
            CommandResult::ok(json!({ "saved": true }))
        }
        Err(error) => {
            error!(error = %error, "批注保存失败");
            CommandResult::error(error)
        }
    }
}

#[tauri::command]
fn emit_domain_event(
    state: State<'_, AppState>,
    event: DomainEvent,
) -> CommandResult<Value> {
    info!(
        event_type = %event.event_type,
        event_id = %event.event_id,
        project_id = %event.project_id,
        "emit_domain_event 触发领域事件"
    );

    let repository = state.repository.lock().expect("repository poisoned");
    if let Err(error) = repository.record_event(&event) {
        error!(error = %error, "事件记录失败");
        return CommandResult::error(error);
    }

    let workflows = match repository.workflows_for_event(&event.project_id, &event.event_type) {
        Ok(value) => value,
        Err(error) => {
            error!(error = %error, "查询工作流失败");
            return CommandResult::error(error);
        }
    };

    let queue = JobQueue::new(&repository);
    let mut queued = 0;
    for workflow in workflows.into_iter().filter(|rule| rule_matches(rule, &event)) {
        for action in &workflow.actions {
            let key = format!(
                "{}:{}:{}",
                event.event_id,
                workflow.id,
                queued
            );
            match queue.enqueue(
                event.project_id.clone(),
                Some(workflow.id.clone()),
                action,
                event.payload.clone(),
                workflow.priority,
                key,
                event.causation_id.clone(),
                1,
            ) {
                Ok(Some(_)) => {
                    queued += 1;
                    debug!(workflow_id = %workflow.id, "工作流任务已入队");
                }
                Ok(None) => {}
                Err(error) => {
                    error!(error = %error, "任务入队失败");
                    return CommandResult::error(error);
                }
            }
        }
    }

    info!(queued = queued, "领域事件处理完成");
    CommandResult::ok(json!({ "recorded": true, "queued": queued }))
}

#[tauri::command]
fn build_context_package(
    project_id: String,
    chapter_id: String,
    revision: u64,
    instruction: String,
    current_scene: String,
    pinned: Vec<String>,
    retrieved: Vec<String>,
    summaries: Vec<String>,
) -> CommandResult<Value> {
    info!(
        project_id = %project_id,
        chapter_id = %chapter_id,
        revision = revision,
        "build_context_package 组装上下文包"
    );
    let package = assemble_context(
        WorkContextRef {
            project_id: project_id.parse().unwrap_or_else(|_| ProjectId::new()),
            branch_id: "main".into(),
            revision: Revision(revision),
            chapter_id: chapter_id.parse().unwrap_or_else(|_| ChapterId::new()),
            block_id: None,
            pov_entity_id: None,
        },
        &instruction,
        &current_scene,
        &pinned,
        &retrieved,
        &summaries,
        AssemblyOptions { token_budget: 12_000 },
    );
    info!(
        sections = package.sections.len(),
        pinned = pinned.len(),
        retrieved = retrieved.len(),
        "上下文包组装完成"
    );
    CommandResult::ok(json!(package))
}

#[tauri::command]
fn run_queue_step(state: State<'_, AppState>) -> CommandResult<Value> {
    debug!("run_queue_step 执行队列任务");
    let repository = state.repository.lock().expect("repository poisoned");
    let runner = JobRunner::new(&repository);
    match runner.run_next() {
        Ok(Some(result)) => {
            info!(
                job_id = %result.job_id,
                operation = %result.operation,
                success = result.success,
                "队列任务执行完成"
            );
            CommandResult::ok(json!({
                "executed": true,
                "jobId": result.job_id,
                "operation": result.operation,
                "success": result.success,
            }))
        }
        Ok(None) => {
            debug!("队列为空，无任务可执行");
            CommandResult::ok(json!({ "executed": false }))
        }
        Err(error) => {
            error!(error = %error, "队列任务执行失败");
            CommandResult::error(error)
        }
    }
}

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
            let repository = Repository::open(database)?;
            app.manage(AppState {
                repository: Mutex::new(repository),
                typing_session: Mutex::new(TypingSession::new(chrono::Utc::now())),
                model_config: Mutex::new(None),
            });
            Ok(())
        })
            .invoke_handler(tauri::generate_handler![
                create_project,
                create_chapter,
                editor_tick,
                context_hints,
                save_model_config,
                load_model_config,
                generate_continuation,
                install_plugin_manifest,
                commit_annotation,
                emit_domain_event,
                build_context_package,
                run_queue_step,
            ])
        .run(tauri::generate_context!())
        .expect("error while running novel agent");
}
