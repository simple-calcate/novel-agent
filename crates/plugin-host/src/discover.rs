use crate::runtime::{PluginInstance, PluginRuntimeError};
use novel_domain::{PluginGrant, PluginManifest, PluginResult, PluginSummary};
use serde::Deserialize;
use serde_json::Value;

const BUNDLED_MANIFESTS: &[&str] = &[
    include_str!("../../../plugins/continuity-checker/plugin.json"),
    include_str!("../../../plugins/summary-extractor/plugin.json"),
    include_str!("../../../plugins/continuation-writer/plugin.json"),
    include_str!("../../../plugins/hello-names/plugin.json"),
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListedManifest {
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    operations: Vec<ListedOperation>,
    #[serde(default)]
    wasm_base64: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListedOperation {
    name: String,
}

pub fn list_bundled_plugins() -> Vec<PluginSummary> {
    let mut items: Vec<PluginSummary> = BUNDLED_MANIFESTS
        .iter()
        .filter_map(|raw| serde_json::from_str::<ListedManifest>(raw).ok())
        .map(|manifest| PluginSummary {
            id: manifest.id,
            name: manifest.name,
            version: manifest.version,
            runtime: if manifest
                .wasm_base64
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                "wasm".into()
            } else {
                "builtin".into()
            },
            operations: manifest
                .operations
                .iter()
                .map(|operation| operation.name.clone())
                .collect(),
        })
        .collect();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    items
}

pub fn bundled_manifest(plugin_id: &str) -> Option<PluginManifest> {
    BUNDLED_MANIFESTS.iter().find_map(|raw| {
        let manifest: PluginManifest = serde_json::from_str(raw).ok()?;
        (manifest.id == plugin_id).then_some(manifest)
    })
}

/// 运行打包插件。清单里带 WASM 时桌面走沙箱；Android 忽略 WASM。
pub fn execute_bundled(
    plugin_id: &str,
    operation: &str,
    input: Value,
) -> Result<PluginResult, PluginRuntimeError> {
    let manifest = bundled_manifest(plugin_id)
        .ok_or_else(|| PluginRuntimeError::OperationNotFound(format!("plugin {plugin_id}")))?;
    let grant = PluginGrant {
        plugin_id: manifest.id.clone(),
        capabilities: manifest.requested_capabilities.clone(),
        enabled: true,
    };
    PluginInstance { manifest, grant }.execute(operation, input)
}
