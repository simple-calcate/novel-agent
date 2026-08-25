//! 插件宿主扩展：`plugin.install` 解析清单并评估权限，
//! `plugin.operation` 分发插件操作（当前为内置执行器，WASM 沙箱后续接入）。

use async_trait::async_trait;
use novel_domain::{PluginGrant, PluginManifest, PluginPlatform};
use novel_kernel::{Extension, KernelBuilder, KernelError, Tool, ToolContext};
use novel_plugin_host::{evaluate, parse_manifest, PluginInstance};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub struct PluginInstallTool;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginInstallInput {
    manifest_json: String,
    /// 已批准的能力（kind 字符串集合）；缺省视为全部批准，便于联调。
    #[serde(default)]
    approved: Vec<String>,
}

fn current_platform() -> PluginPlatform {
    if cfg!(target_os = "android") {
        PluginPlatform::Android
    } else if cfg!(target_os = "windows") {
        PluginPlatform::Windows
    } else {
        PluginPlatform::Linux
    }
}

#[async_trait]
impl Tool for PluginInstallTool {
    fn id(&self) -> &str {
        "plugin.install"
    }

    fn summary(&self) -> &str {
        "解析插件清单并评估权限交集"
    }

    async fn execute(&self, input: Value, _ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let parsed: PluginInstallInput =
            serde_json::from_value(input).map_err(|error| KernelError::ToolFailed {
                tool: "plugin.install".into(),
                message: format!("无效的安装参数: {error}"),
            })?;

        let manifest =
            parse_manifest(&parsed.manifest_json, current_platform()).map_err(|error| {
                KernelError::ToolFailed {
                    tool: "plugin.install".into(),
                    message: error.to_string(),
                }
            })?;

        let approved: BTreeSet<String> = parsed.approved.into_iter().collect();
        let approved_capabilities = if approved.is_empty() {
            manifest.requested_capabilities.clone()
        } else {
            manifest
                .requested_capabilities
                .iter()
                .filter(|capability| approved.contains(&capability_kind(capability)))
                .cloned()
                .collect()
        };
        let decision = evaluate(&manifest, &approved_capabilities);
        Ok(json!({
            "manifest": manifest,
            "granted": decision.granted,
            "denied": decision.denied,
        }))
    }
}

fn capability_kind(capability: &novel_domain::Capability) -> String {
    serde_json::to_value(capability)
        .ok()
        .and_then(|value| value.get("kind").cloned())
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown".into())
}

pub struct PluginOperationTool;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginOperationInput {
    manifest: PluginManifest,
    grant: PluginGrant,
    operation: String,
    #[serde(default)]
    input: Value,
}

#[async_trait]
impl Tool for PluginOperationTool {
    fn id(&self) -> &str {
        "plugin.operation"
    }

    fn summary(&self) -> &str {
        "执行一个已安装插件的操作"
    }

    async fn execute(&self, payload: Value, _ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let parsed: PluginOperationInput =
            serde_json::from_value(payload).map_err(|error| KernelError::ToolFailed {
                tool: "plugin.operation".into(),
                message: format!("无效的操作参数: {error}"),
            })?;
        let instance = PluginInstance {
            manifest: parsed.manifest,
            grant: parsed.grant,
        };
        let result = instance
            .execute_builtin(&parsed.operation, parsed.input)
            .map_err(|error| KernelError::ToolFailed {
                tool: "plugin.operation".into(),
                message: error.to_string(),
            })?;
        Ok(json!(result))
    }
}

/// 插件宿主扩展。
pub struct PluginHostExtension;

impl Extension for PluginHostExtension {
    fn id(&self) -> &str {
        "builtin.plugin-host"
    }

    fn setup(&self, builder: &mut KernelBuilder) -> Result<(), KernelError> {
        builder.register_tool(PluginInstallTool);
        builder.register_tool(PluginOperationTool);
        Ok(())
    }
}
