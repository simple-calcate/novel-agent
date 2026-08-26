use novel_domain::{Capability, PluginPlatform};
use novel_plugin_host::{evaluate, grant_intersection, parse_manifest};
use std::collections::BTreeSet;

const MANIFEST: &str = r#"{
  "id": "test-plugin",
  "name": "测试插件",
  "version": "0.1.0",
  "apiVersion": 1,
  "platforms": ["linux"],
  "operations": [
    {
      "name": "test-op",
      "inputSchema": {"type": "object"},
      "outputSchema": {"type": "object"},
      "triggers": ["editor.idle"]
    }
  ],
  "requestedCapabilities": [
    {"kind": "readSelection"},
    {"kind": "log"}
  ]
}"#;

#[test]
fn parse_valid_manifest() {
    let manifest = parse_manifest(MANIFEST, PluginPlatform::Linux).unwrap();
    assert_eq!(manifest.name, "测试插件");
    assert_eq!(manifest.operations.len(), 1);
}

#[test]
fn reject_unsupported_platform() {
    let result = parse_manifest(MANIFEST, PluginPlatform::Android);
    assert!(result.is_err());
}

#[test]
fn capability_grant_intersection() {
    let manifest = parse_manifest(MANIFEST, PluginPlatform::Linux).unwrap();
    let approved: BTreeSet<Capability> = [Capability::Log].into_iter().collect();
    let grant = grant_intersection(&manifest, &approved);
    assert!(grant.capabilities.contains(&Capability::Log));
    assert!(!grant.capabilities.contains(&Capability::ReadSelection));
}

#[test]
fn evaluate_denies_unapproved() {
    let manifest = parse_manifest(MANIFEST, PluginPlatform::Linux).unwrap();
    let approved: BTreeSet<Capability> = [Capability::Log].into_iter().collect();
    let decision = evaluate(&manifest, &approved);
    assert!(decision.granted.contains(&Capability::Log));
    assert!(decision.denied.contains(&Capability::ReadSelection));
}

#[test]
fn bundled_plugins_are_embedded() {
    let plugins = novel_plugin_host::list_bundled_plugins();
    assert_eq!(plugins.len(), 3);
    assert!(plugins
        .iter()
        .any(|plugin| plugin.id == "continuity-checker"));
    assert!(plugins.iter().all(|plugin| plugin.runtime == "builtin"));
}
