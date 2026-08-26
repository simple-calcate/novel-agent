use novel_domain::{PluginManifest, PluginSummary};

const BUNDLED_MANIFESTS: &[&str] = &[
    include_str!("../../../plugins/continuity-checker/plugin.json"),
    include_str!("../../../plugins/summary-extractor/plugin.json"),
    include_str!("../../../plugins/continuation-writer/plugin.json"),
];

pub fn list_bundled_plugins() -> Vec<PluginSummary> {
    let mut items: Vec<PluginSummary> = BUNDLED_MANIFESTS
        .iter()
        .filter_map(|raw| serde_json::from_str::<PluginManifest>(raw).ok())
        .map(|manifest| PluginSummary {
            id: manifest.id,
            name: manifest.name,
            version: manifest.version,
            runtime: "builtin".into(),
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
