//! 编辑器会话相关命令：输入心跳、上下文提示、批注、领域事件与 AI 续写。

use crate::common::{notify_if_queued, workspace, CommandResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, State};
use tracing::{debug, error, info};

use novel_domain::{Annotation, ContentPatch, DomainEvent, Revision};

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

#[tauri::command]
pub fn editor_tick(
    state: State<'_, crate::AppState>,
    input: EditorTickInput,
) -> CommandResult<Value> {
    debug!(
        revision = input.revision,
        chars = input.chars_since_commit,
        composing = input.composing,
        "editor_tick"
    );
    // 锁中毒时恢复旧数据继续跑，好过把整个 app 崩掉
    let mut typing = state
        .typing_session
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
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
pub async fn context_hints(
    state: State<'_, crate::AppState>,
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
pub fn commit_annotation(
    state: State<'_, crate::AppState>,
    annotation: Annotation,
) -> CommandResult<Value> {
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
pub fn emit_domain_event<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, crate::AppState>,
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

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn build_context_package(
    state: State<'_, crate::AppState>,
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
pub async fn generate_continuation(
    state: State<'_, crate::AppState>,
    chapter_id: String,
    revision: u64,
    prompt: String,
    context_text: String,
    config: Option<crate::commands::settings::ModelConfigInput>,
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
