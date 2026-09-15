# crates/storage

包名 `novel-storage`。单写者 SQLite。切片：[docs/wiki/agents.md](../../docs/wiki/agents.md)。

- 入口：`src/handle.rs`（`StorageHandle`）。同线程嵌套 `with`/`execute` → `Reentrancy`。**禁止**持锁时 `kernel.dispatch`。
- 新表：`migrations/00xx_*.sql` **并且**登记 `src/migrations.rs` 的 `MIGRATIONS`。
- 仓储：`src/repository/`。`story_entries`（`structure.rs`）≠ `canon_*`（`canon.rs`）。
- `scenes.pov_entity_id` 列 = 领域 `povEntryId`，指向结构人物，不是正史实体。
- 写路径用 `write_with_outbox`；载荷只带 id / 修订号。
- `export.rs` 能拼 TXT/MD，**没有** UI/command。
- `app_settings` 不存 API Key。
- 测试：`tests/`（`repository_test`、`structure_test`、`commit_patch_test`、`outbox_test`、`canon_test`、`migration_test`、`queue_state_test`）。
