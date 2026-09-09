//! 命令层共享管道：统一返回结构、workspace 访问、ID 解析与队列事件通知。

use crate::AppState;
use novel_domain::{BookId, ChapterId, FactId, FactStatus, ProjectId, SceneId, StoryEntryKind, VolumeId};
use novel_extensions::Workspace;
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tracing::warn;

pub fn workspace(state: &AppState) -> Workspace<'_> {
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
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(error: impl ToString) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error.to_string()),
        }
    }

    pub fn from_result(result: Result<T, impl ToString>) -> Self {
        match result {
            Ok(data) => Self::ok(data),
            Err(error) => Self::error(error),
        }
    }
}

/// 通知前端队列有活动（入队或任务执行完成），由前端接管后续 drain。
/// 前端以此替代固定轮询：空闲时零 IPC、零日志。
pub fn notify_queue_changed<R: tauri::Runtime>(app: &AppHandle<R>) {
    let payload = json!({ "at": chrono::Utc::now().to_rfc3339() });
    if let Err(err) = app.emit("queue:changed", payload) {
        warn!(error = %err, "通知前端队列事件失败");
    }
}

pub fn notify_if_queued<R: tauri::Runtime>(app: &AppHandle<R>, queued: u64) {
    if queued > 0 {
        notify_queue_changed(app);
    }
}

pub fn parse_project_id(value: &str) -> Result<ProjectId, String> {
    value
        .parse()
        .map_err(|_| format!("invalid project id: {value}"))
}

pub fn parse_book_id(value: &str) -> Result<BookId, String> {
    value
        .parse()
        .map_err(|_| format!("invalid book id: {value}"))
}

pub fn parse_volume_id(value: &str) -> Result<VolumeId, String> {
    value
        .parse()
        .map_err(|_| format!("invalid volume id: {value}"))
}

pub fn parse_scene_id(value: &str) -> Result<SceneId, String> {
    value
        .parse()
        .map_err(|_| format!("invalid scene id: {value}"))
}

pub fn parse_chapter_id(value: &str) -> Result<ChapterId, String> {
    value
        .parse()
        .map_err(|_| format!("invalid chapter id: {value}"))
}

pub fn parse_fact_id(value: &str) -> Result<FactId, String> {
    value
        .parse()
        .map_err(|_| format!("invalid fact id: {value}"))
}

pub fn parse_fact_status(value: Option<&str>) -> Result<Option<FactStatus>, String> {
    match value {
        None | Some("") => Ok(None),
        Some("candidate") => Ok(Some(FactStatus::Candidate)),
        Some("accepted") => Ok(Some(FactStatus::Accepted)),
        Some("rejected") => Ok(Some(FactStatus::Rejected)),
        Some("superseded") => Ok(Some(FactStatus::Superseded)),
        Some(other) => Err(format!("invalid fact status: {other}")),
    }
}

pub fn parse_story_kind(value: &str) -> Result<StoryEntryKind, String> {
    match value {
        "character" => Ok(StoryEntryKind::Character),
        "setting" => Ok(StoryEntryKind::Setting),
        "foreshadow" => Ok(StoryEntryKind::Foreshadow),
        other => Err(format!("invalid story entry kind: {other}")),
    }
}
