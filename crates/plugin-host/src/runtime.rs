use novel_domain::{Capability, PluginGrant, PluginManifest, PluginResult};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginRuntimeError {
    #[error("operation not found: {0}")]
    OperationNotFound(String),
    #[error("capability denied: {0:?}")]
    CapabilityDenied(Capability),
    #[error("invalid output: {0}")]
    InvalidOutput(String),
}

pub struct PluginInstance {
    pub manifest: PluginManifest,
    pub grant: PluginGrant,
}

impl PluginInstance {
    pub fn execute_builtin(
        &self,
        operation: &str,
        input: Value,
    ) -> Result<PluginResult, PluginRuntimeError> {
        let declaration = self
            .manifest
            .operations
            .iter()
            .find(|item| item.name == operation)
            .ok_or_else(|| PluginRuntimeError::OperationNotFound(operation.into()))?;

        if !declaration.triggers.is_empty()
            && !self
                .grant
                .capabilities
                .contains(&Capability::ReadSelection)
        {
            return Err(PluginRuntimeError::CapabilityDenied(Capability::ReadSelection));
        }

        Ok(PluginResult {
            output: serde_json::json!({
                "operation": operation,
                "input": input,
                "message": "内置执行器已接收操作；WASM 运行时将在桌面宿主中接入"
            }),
            logs: vec![format!("executed {}", operation)],
        })
    }
}
