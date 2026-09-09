//! 插件发现与执行命令。

use crate::common::{workspace, CommandResult};
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::State;
use tracing::{error, info};

use novel_domain::{PluginResult, PluginSummary};

#[tauri::command]
pub fn list_plugins(state: State<'_, crate::AppState>) -> CommandResult<Vec<PluginSummary>> {
    CommandResult::ok(workspace(&state).list_plugins())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunPluginInput {
    pub plugin_id: String,
    pub operation: String,
    #[serde(default)]
    pub input: Value,
}

#[tauri::command]
pub fn run_plugin_operation(
    state: State<'_, crate::AppState>,
    input: RunPluginInput,
) -> CommandResult<PluginResult> {
    CommandResult::from_result(workspace(&state).run_plugin_operation(
        &input.plugin_id,
        &input.operation,
        input.input,
    ))
}

#[tauri::command]
pub async fn install_plugin_manifest(
    state: State<'_, crate::AppState>,
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
