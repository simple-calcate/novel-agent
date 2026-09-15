# crates/plugin-host

包名 `novel-plugin-host`。清单、权限、桌面 wasmi。

- `discover.rs`：打包 `plugins/*/plugin.json`（`include_str!`）。
- `manifest.rs` / `permissions.rs` / `runtime.rs`。
- `sandbox.rs`：非 Android 才编译。无 WASI、无宿主导入。请求 JSON 写在已有线性内存之后。
- Android 忽略 `wasmBase64`，走内置占位。
- 只有 `hello-names` 是可运行 WASM；其余打包项是占位回执。
- 测试：`tests/manifest_test.rs`、`sandbox_test.rs`。
