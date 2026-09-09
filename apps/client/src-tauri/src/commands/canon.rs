//! 正史（canon）抽取/审核与故事设定条目命令。

use crate::common::{
    parse_chapter_id, parse_fact_id, parse_fact_status, parse_project_id, parse_story_kind,
    workspace, CommandResult,
};
use tauri::State;

use novel_domain::{CanonProposal, StoryEntry};

#[tauri::command]
pub fn propose_canon(
    state: State<'_, crate::AppState>,
    chapter_id: String,
) -> CommandResult<Vec<CanonProposal>> {
    let chapter_id = match parse_chapter_id(&chapter_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).propose_canon_from_chapter(&chapter_id))
}

#[tauri::command]
pub fn list_canon(
    state: State<'_, crate::AppState>,
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
pub fn review_canon_fact(
    state: State<'_, crate::AppState>,
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
pub fn create_story_entry(
    state: State<'_, crate::AppState>,
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
pub fn list_story_entries(
    state: State<'_, crate::AppState>,
    project_id: String,
) -> CommandResult<Vec<StoryEntry>> {
    let project_id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).list_story_entries(&project_id))
}

#[tauri::command]
pub fn delete_story_entry(
    state: State<'_, crate::AppState>,
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
