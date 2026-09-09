//! outbox 同步日志命令：本地 JSONL 追加写。

use crate::common::{workspace, CommandResult};
use novel_extensions::OutboxFlushResult;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn pending_outbox_count(state: State<'_, crate::AppState>) -> CommandResult<u32> {
    CommandResult::from_result(workspace(&state).pending_outbox_count())
}

#[tauri::command]
pub async fn flush_outbox_journal(
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<CommandResult<OutboxFlushResult>, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let path = data_dir.join("sync").join("outbox-journal.jsonl");
    // 万条读取 + fsync 是重阻塞，放到 blocking 池，别卡 async 运行时
    let kernel = state.kernel.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        novel_extensions::Workspace::new(&kernel).flush_outbox_journal(path)
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(CommandResult::from_result(result))
}
