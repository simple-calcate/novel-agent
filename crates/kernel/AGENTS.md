# crates/kernel

包名 `novel-kernel`。只做四件事：组请求、按预算截流、分发工具、派事件。

- `lib.rs`：`Kernel` / `KernelBuilder` / `Extension`。
- `tool.rs`：`Tool` + `ToolContext`。工具禁止依赖 Tauri。
- `agent.rs` / `budget.rs` / `provider.rs` / `events.rs` / `services.rs`。
- **不要**加 rusqlite / reqwest。业务进 `novel-extensions`。
- 测试：`tests/kernel_test.rs`。
