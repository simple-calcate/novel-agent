# crates/extensions

包名 `novel-extensions`。内置扩展 + `Workspace`。切片：[docs/wiki/agents.md](../../docs/wiki/agents.md)。

- 用户点一下 → 加 `workspace.rs` 方法，不要先做成 Tool。
- Agent/队列/浮带 → `src/` 里对应 Tool，在 `lib.rs` `BuiltinsExtension` 注册（或覆盖同名）。
- 访问 SQLite 只走 `util.rs` 的 `with_repository`。写完再 `dispatch`。
- `secrets.rs`：`SecretVault`。测试用 `SecretVault::memory()`。
- `hints.rs` 调 `HintEngine::rank_entries`（结构条目）。`context.rs` 是装配，不是预选条。
- 测试：`tests/workspace_test.rs`、`queue_flow_test.rs`、`manual_enqueue_test.rs`、`block_mode_event_test.rs`、`provider_http_test.rs`。
