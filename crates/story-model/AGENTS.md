# crates/story-model

包名 `novel-story-model`。启发式正史、故事图、连续性。

- **不是**写作主路径。界面不用抽取/审核闭环。见 ADR 0009。
- 不要把这里的实体接到 `ContextRail` / `story_entries`。
- `extract.rs` / `graph.rs` / `snapshot.rs` / `validators.rs`。
- 顶栏「检查」入队的是 `continuity.check`，走这里，不是结构预选条。
- 测试：`tests/extract_test.rs`、`snapshot_test.rs`。
