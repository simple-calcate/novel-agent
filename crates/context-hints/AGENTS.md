# crates/context-hints

包名 `novel-context-hints`。预选条匹配。产品路径是 `StoryEntry`，不是 canon。

- `src/lib.rs`：`HintEngine::rank_entries`。
- `src/entry_match.rs`：全名 / 别名 / 词核 / 说明关键词 / dwell / 词汇检索。
- **必须同时改** `apps/client/src/structure/match.ts`，并更新 `packages/match-fixtures/cases.json`。
- Rust 测试 `include_str!` 那份 JSON：`tests/hints_test.rs`。
- `rank()` 还有一条旧的实体/伏笔匹配，**不是**编辑器预选条数据源。
