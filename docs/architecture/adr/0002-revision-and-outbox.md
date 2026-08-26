# ADR 0002: Revision + OperationLog + Outbox

## 状态
已接受

## 背景
需要在单作者多设备场景下保证数据一致性，同时避免首版引入 CRDT 的复杂度。

## 决策
- 每章使用单调递增的 Revision。
- 所有写操作经过 `StorageHandle`（同线程禁止重入，见 [ADR 0008](0008-workspace-storage-handle.md)）。
- 业务状态、operation log、任务和 outbox 在同一事务内提交。
- 同步传输幂等 outbox 变更，冲突时保留冲突副本。

## 后果
- 崩溃后可恢复到一致状态。
- 作品库、修订、任务入队、结构条目的写路径会在同一 SQLite 事务里插入 `outbox` 行；`list_pending_outbox` / `mark_outbox_delivered` 供同步消费者使用。
- 同步传输仍是阶段 2（见 `docs/sync-and-cloud.md`）：本机只保证变更已入队，不发送。
- 冲突解决逻辑简单可靠。
- 多人实时协作需要后续升级为 Yjs/CRDT。
