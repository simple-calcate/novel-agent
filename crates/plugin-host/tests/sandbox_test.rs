#![cfg(not(target_os = "android"))]

use novel_domain::{Capability, PluginGrant, PluginManifest, PluginOperation, PluginPlatform};
use novel_plugin_host::PluginInstance;
use serde_json::json;
use std::collections::BTreeSet;

const ECHO_WAT: &str = r#"(module
  (memory (export "memory") 1)
  (func (export "plugin_execute") (param i32 i32) (result i32 i32)
    local.get 0
    local.get 1)
)"#;

const LOOP_WAT: &str = r#"(module
  (memory (export "memory") 1)
  (func (export "plugin_execute") (param i32 i32) (result i32 i32)
    (loop $forever
      br $forever)
    local.get 0
    local.get 1)
)"#;

const WASI_WAT: &str = r#"(module
  (import "wasi_snapshot_preview1" "proc_exit" (func (param i32)))
  (memory (export "memory") 1)
  (func (export "plugin_execute") (param i32 i32) (result i32 i32)
    local.get 0
    local.get 1)
)"#;

fn encode_wat(wat: &str) -> String {
    let wasm = wat::parse_str(wat).expect("valid wat");
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, wasm)
}

fn instance(wasm_base64: Option<String>) -> PluginInstance {
    PluginInstance {
        manifest: PluginManifest {
            id: "echo".into(),
            name: "回声".into(),
            version: "0.1.0".into(),
            api_version: 1,
            platforms: vec![PluginPlatform::Linux],
            operations: vec![PluginOperation {
                name: "ping".into(),
                input_schema: json!({"type": "object"}),
                output_schema: json!({"type": "object"}),
                triggers: vec![],
            }],
            requested_capabilities: BTreeSet::new(),
            settings_schema: json!({}),
            wasm_base64,
        },
        grant: PluginGrant {
            plugin_id: "echo".into(),
            capabilities: BTreeSet::from([Capability::Log]),
            enabled: true,
        },
    }
}

#[test]
fn wasm_echo_guest_runs_in_sandbox() {
    let plugin = instance(Some(encode_wat(ECHO_WAT)));
    let result = plugin
        .execute("ping", json!({"hello": "雾港"}))
        .expect("sandbox execute");
    assert_eq!(result.output["operation"], "ping");
    assert_eq!(result.output["input"]["hello"], "雾港");
    assert!(result.logs.iter().any(|line| line == "wasm"));
}

#[test]
fn missing_wasm_falls_back_to_builtin() {
    let plugin = instance(None);
    let result = plugin.execute("ping", json!({})).unwrap();
    assert!(result.output["message"].as_str().unwrap().contains("内置"));
}

#[test]
fn unknown_operation_is_rejected_before_wasm() {
    let plugin = instance(Some(encode_wat(ECHO_WAT)));
    let error = plugin.execute("missing", json!({})).unwrap_err();
    assert!(error.to_string().contains("operation not found"));
}

#[test]
fn fuel_limit_stops_infinite_loop() {
    let plugin = instance(Some(encode_wat(LOOP_WAT)));
    let error = plugin.execute("ping", json!({})).unwrap_err();
    assert!(
        error.to_string().contains("trap") || error.to_string().contains("fuel"),
        "{error}"
    );
}

#[test]
fn missing_wasi_import_is_rejected() {
    let plugin = instance(Some(encode_wat(WASI_WAT)));
    let error = plugin.execute("ping", json!({})).unwrap_err();
    assert!(
        error.to_string().contains("instantiate") || error.to_string().contains("import"),
        "{error}"
    );
}
