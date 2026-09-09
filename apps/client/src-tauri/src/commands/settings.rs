//! 模型配置与写作偏好（反馈记忆）命令。

use crate::common::{parse_project_id, workspace, CommandResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::State;
use tracing::{debug, error, info};

use novel_kernel::ProviderConfig;

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
    pub fn provider_config(&self) -> ProviderConfig {
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

#[tauri::command]
pub async fn save_model_config(
    state: State<'_, crate::AppState>,
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
pub fn load_model_config(state: State<'_, crate::AppState>) -> Result<Value, String> {
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
pub fn record_generation_feedback(
    state: State<'_, crate::AppState>,
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
pub fn list_preferences(
    state: State<'_, crate::AppState>,
    project_id: String,
) -> CommandResult<Vec<novel_domain::PreferenceRule>> {
    let project_id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    CommandResult::from_result(workspace(&state).list_preference_rules(&project_id))
}

#[tauri::command]
pub fn set_preference_status(
    state: State<'_, crate::AppState>,
    project_id: String,
    rule_id: String,
    disabled: bool,
) -> CommandResult<Vec<novel_domain::PreferenceRule>> {
    let project_id = match parse_project_id(&project_id) {
        Ok(id) => id,
        Err(err) => return CommandResult::error(err),
    };
    let rule_id = match rule_id.parse() {
        Ok(id) => id,
        Err(_) => return CommandResult::error(format!("invalid preference id: {rule_id}")),
    };
    CommandResult::from_result(workspace(&state).set_preference_status(
        &project_id,
        &rule_id,
        disabled,
    ))
}
