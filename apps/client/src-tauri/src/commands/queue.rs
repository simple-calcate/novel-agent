//! 队列与内核自描述命令。队列由前端事件驱动执行（`queue:changed` → drain）。

use crate::common::{notify_queue_changed, workspace, CommandResult};
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::{AppHandle, State};
use tracing::{error, info, warn};

use novel_domain::JobView;
use novel_kernel::ToolDescriptor;

#[tauri::command]
pub async fn run_queue_step<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, crate::AppState>,
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
pub fn kernel_tools(state: State<'_, crate::AppState>) -> CommandResult<Value> {
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
pub fn enqueue_job<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, crate::AppState>,
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
pub fn list_jobs(state: State<'_, crate::AppState>) -> CommandResult<Vec<JobView>> {
    CommandResult::from_result(workspace(&state).list_jobs(30))
}
