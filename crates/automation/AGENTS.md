# crates/automation

包名 `novel-automation`。信号、工作流匹配、队列**状态机**。真正执行在 `novel-extensions` 的 `queue.tick`。

- `signals.rs`：`TypingSession` / `editor.idle` 等。
- `workflow.rs`：规则匹配。
- `queue.rs`：领取、退避、死信、回收陈旧 running。
- 不要在这里调模型或写 UI。
- 测试：`tests/workflow_test.rs`、`queue_test.rs`。
