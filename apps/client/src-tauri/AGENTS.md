# apps/client/src-tauri

包名 `novel-agent-client`（lib `novel_agent_lib`）。只译 JSON，不写业务。

- `src/lib.rs`：装配 `StorageHandle` + `SecretVault` + `BuiltinsExtension`；**`generate_handler!` 必须与 [interfaces.md](../../../docs/interfaces.md) §5 一致**。
- `src/commands/`：按领域拆。结构条目命令在 `canon.rs`，表仍是 `story_entries`。
- `src/common.rs`：`CommandResult { ok, data, error }`、`workspace()`、`queue:changed`。
- `src/main.rs` 只调 `novel_agent_lib::run()`。
- 测试：`src/command_tests.rs`（`cargo test -p novel-agent-client`）。
- 新增 command：模块 → handler → interfaces §5 → `libraryApi`（若 UI 要调）。
