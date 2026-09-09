//! 书架/卷/章/场景结构与章节正文的命令。

use crate::common::{
    notify_if_queued, parse_book_id, parse_chapter_id, parse_project_id, parse_scene_id,
    parse_volume_id, workspace, CommandResult,
};
use serde::Deserialize;
use tauri::State;
use tracing::{info, warn};

use novel_domain::{
    Book, Chapter, ChapterBody, ContentBlock, DomainEvent, EventId, EventSource, LibrarySnapshot,
    Project, Scene, Volume, EVENT_SCHEMA_VERSION,
};

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
pub struct NewSceneInput {
    pub project_id: String,
    pub chapter_id: String,
    pub title: String,
    #[serde(default)]
    pub position: u32,
    #[serde(default)]
    pub pov_entry_id: Option<String>,
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

#[tauri::command]
pub fn create_project(
    state: State<'_, crate::AppState>,
    input: NewProjectInput,
) -> CommandResult<Project> {
    info!(title = %input.title, "create_project 调用");
    CommandResult::from_result(workspace(&state).create_project(&input.title))
}

#[tauri::command]
pub fn create_book(
    state: State<'_, crate::AppState>,
    input: NewBookInput,
) -> CommandResult<Book> {
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
pub fn create_chapter(
    state: State<'_, crate::AppState>,
    input: NewChapterInput,
) -> CommandResult<Chapter> {
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
pub fn create_volume(
    state: State<'_, crate::AppState>,
    input: NewVolumeInput,
) -> CommandResult<Volume> {
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
pub fn create_scene(
    state: State<'_, crate::AppState>,
    input: NewSceneInput,
) -> CommandResult<Scene> {
    let project_id = match parse_project_id(&input.project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    if parse_chapter_id(&input.chapter_id).is_err() {
        return CommandResult::error("invalid chapter id");
    }
    CommandResult::from_result(workspace(&state).create_scene(
        &project_id,
        &input.chapter_id,
        &input.title,
        input.position,
        input.pov_entry_id.as_deref(),
    ))
}

#[tauri::command]
pub fn rename_scene(
    state: State<'_, crate::AppState>,
    project_id: String,
    scene_id: String,
    title: String,
) -> CommandResult<LibrarySnapshot> {
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let sid = match parse_scene_id(&scene_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).rename_scene(&pid, &sid, &title))
}

#[tauri::command]
pub fn set_scene_pov(
    state: State<'_, crate::AppState>,
    project_id: String,
    scene_id: String,
    pov_entry_id: Option<String>,
) -> CommandResult<LibrarySnapshot> {
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let sid = match parse_scene_id(&scene_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).set_scene_pov(&pid, &sid, pov_entry_id.as_deref()))
}

#[tauri::command]
pub fn delete_scene(
    state: State<'_, crate::AppState>,
    project_id: String,
    scene_id: String,
) -> CommandResult<LibrarySnapshot> {
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let sid = match parse_scene_id(&scene_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).delete_scene(&pid, &sid))
}

#[tauri::command]
pub fn move_scene(
    state: State<'_, crate::AppState>,
    project_id: String,
    scene_id: String,
    delta: i32,
) -> CommandResult<LibrarySnapshot> {
    let pid = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let sid = match parse_scene_id(&scene_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).move_scene(&pid, &sid, delta))
}

#[tauri::command]
pub fn load_library(
    state: State<'_, crate::AppState>,
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
pub fn set_active_project(
    state: State<'_, crate::AppState>,
    project_id: String,
) -> CommandResult<LibrarySnapshot> {
    let project_id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).set_active_project(project_id))
}

#[tauri::command]
pub fn load_chapter(
    state: State<'_, crate::AppState>,
    chapter_id: String,
) -> CommandResult<ChapterBody> {
    let id = match parse_chapter_id(&chapter_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).load_chapter(&id))
}

#[tauri::command]
pub fn save_chapter(
    state: State<'_, crate::AppState>,
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
pub fn rename_project(
    state: State<'_, crate::AppState>,
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
pub fn delete_project(
    state: State<'_, crate::AppState>,
    project_id: String,
) -> CommandResult<LibrarySnapshot> {
    let id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).delete_project(&id))
}

#[tauri::command]
pub fn rename_book(
    state: State<'_, crate::AppState>,
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
pub fn delete_book(
    state: State<'_, crate::AppState>,
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
pub fn move_book(
    state: State<'_, crate::AppState>,
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
pub fn rename_volume(
    state: State<'_, crate::AppState>,
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
pub fn delete_volume(
    state: State<'_, crate::AppState>,
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
pub fn move_volume(
    state: State<'_, crate::AppState>,
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
pub fn rename_chapter(
    state: State<'_, crate::AppState>,
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
pub fn delete_chapter(
    state: State<'_, crate::AppState>,
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
pub fn move_chapter(
    state: State<'_, crate::AppState>,
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

/// 思考/正文模式切换信号量：
/// 前端在"新行行首"状态按 Tab 切换块类型时调用，构造领域事件
/// `block.mode.changed` 并 dispatch——工作流规则可监听该事件，
/// 匹配后自动入队对应的任务序列（插件扩展点）。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn emit_block_mode_changed<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, crate::AppState>,
    project_id: String,
    chapter_id: String,
    mode: String,
    previous_mode: String,
    block_id: Option<String>,
    position: Option<u32>,
) -> CommandResult<serde_json::Value> {
    use std::str::FromStr;
    use novel_domain::{BlockId, ChapterId, ProjectId};
    use serde_json::json;
    use tracing::error;

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
        error!(error = %err, "模式切换事件处理失败");
        return CommandResult::error(err);
    }
    let queued = summary.queued_count();
    info!(queued = queued, "模式切换事件处理完成");
    notify_if_queued(&app, queued);
    CommandResult::ok(json!({ "recorded": true, "queued": queued }))
}
