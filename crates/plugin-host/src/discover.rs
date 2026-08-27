use novel_domain::PluginSummary;
use serde::Deserialize;

const BUNDLED_MANIFESTS: &[&str] = &[
    include_str!("../../../plugins/continuity-checker/plugin.json"),
    include_str!("../../../plugins/summary-extractor/plugin.json"),
    include_str!("../../../plugins/continuation-writer/plugin.json"),
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
