# crates/feedback-memory

包名 `novel-feedback-memory`。拒绝续写 → 偏好规则。无 IO。

- 落库在 `novel-storage` `repository/feedback.rs`。
- Workspace 在拒绝后续写时把未停用规则拼进 system prompt。
- 同一规则文本拒绝两次升为 Confirmed。停用后不进提示。
- 测试：`tests/feedback_test.rs`。
