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

const PACKED_I64_WAT: &str = r#"(module
  (memory (export "memory") 1)
  (func (export "plugin_execute") (param i32 i32) (result i64)
    (i64.or
      (i64.shl (i64.extend_i32_u (local.get 0)) (i64.const 32))
      (i64.extend_i32_u (local.get 1))))
)"#;

const KEEP_STATIC_WAT: &str = r#"(module
  (memory (export "memory") 1)
  (data (i32.const 0) "KEEPME")
  (func (export "plugin_execute") (param $ptr i32) (param $len i32) (result i32 i32)
    (if (i32.or
          (i32.ne (i32.load8_u (i32.const 0)) (i32.const 75))
          (i32.ne (i32.load8_u (i32.const 5)) (i32.const 69)))
      (then unreachable))
    local.get $ptr
    local.get $len)
)"#;

fn encode_wat(wat: &str) -> String {
    let wasm = wat::parse_str(wat).expect("valid wat");
    encode_bytes(&wasm)
}

fn encode_bytes(wasm: &[u8]) -> String {
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

#[test]
fn packed_i64_return_is_accepted() {
    let plugin = instance(Some(encode_wat(PACKED_I64_WAT)));
    let result = plugin
        .execute("ping", json!({"hello": "雾港"}))
        .expect("packed i64 execute");
    assert_eq!(result.output["operation"], "ping");
    assert_eq!(result.output["input"]["hello"], "雾港");
}

#[test]
fn host_does_not_clobber_guest_static_data() {
    let plugin = instance(Some(encode_wat(KEEP_STATIC_WAT)));
    let result = plugin
        .execute("ping", json!({"hello": "雾港"}))
        .expect("static data preserved");
    assert_eq!(result.output["input"]["hello"], "雾港");
}

/// 由 `pnpm --filter @novel-agent/plugin-compile compile:hello-names` 写入 `plugins/hello-names/plugin.json`。
#[test]
fn compiled_hello_names_counts_in_wasmi() {
    let result = novel_plugin_host::execute_bundled(
        "hello-names",
        "count-names",
        json!({
            "selection": "林晚走进雾港，林晚没有回头",
            "names": ["林晚", "雾儿"]
        }),
    )
    .expect("compiled guest");
    assert_eq!(result.output["counts"]["林晚"], 2);
    assert_eq!(result.output["counts"]["雾儿"], 0);
    assert!(result.logs.iter().any(|line| line == "hello-names"));
}
