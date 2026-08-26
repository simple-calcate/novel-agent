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
    #[error("wasm sandbox: {0}")]
    Sandbox(String),
}

pub struct PluginInstance {
    pub manifest: PluginManifest,
    pub grant: PluginGrant,
}

impl PluginInstance {
    pub fn authorize(&self, operation: &str) -> Result<(), PluginRuntimeError> {
        let declaration = self
            .manifest
            .operations
            .iter()
            .find(|item| item.name == operation)
            .ok_or_else(|| PluginRuntimeError::OperationNotFound(operation.into()))?;

        if !declaration.triggers.is_empty()
            && !self.grant.capabilities.contains(&Capability::ReadSelection)
        {
            return Err(PluginRuntimeError::CapabilityDenied(
                Capability::ReadSelection,
            ));
        }
        Ok(())
    }

    /// 有 WASM 字节且当前平台支持沙箱时走 wasmi；否则走内置执行器。
    pub fn execute(
        &self,
        operation: &str,
        input: Value,
    ) -> Result<PluginResult, PluginRuntimeError> {
        #[cfg(not(target_os = "android"))]
        if self
            .manifest
            .wasm_base64
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            return crate::sandbox::execute(self, operation, input);
        }
        self.execute_builtin(operation, input)
    }

    pub fn execute_builtin(
        &self,
        operation: &str,
        input: Value,
    ) -> Result<PluginResult, PluginRuntimeError> {
        self.authorize(operation)?;
        Ok(PluginResult {
            output: serde_json::json!({
                "operation": operation,
                "input": input,
                "message": "内置执行器已接收操作；未提供 WASM 模块或当前平台不支持沙箱"
            }),
            logs: vec![format!("executed {}", operation)],
        })
    }
}
