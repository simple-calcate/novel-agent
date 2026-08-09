# ADR 0002: Revision + OperationLog + Outbox

## 状态
已接受

## 背景
需要在单作者多设备场景下保证数据一致性，同时避免首版引入 CRDT 的复杂度。

## 决策
- 每章使用单调递增的 Revision。
- 所有写操作经过 Rust 单写者 actor。
- 业务状态、operation log、任务和 outbox 在同一事务内提交。
- 同步传输幂等 outbox 变更，冲突时保留冲突副本。

## 后果
- 崩溃后可恢复到一致状态。
- 冲突解决逻辑简单可靠。
- 多人实时协作需要后续升级为 Yjs/CRDT。
