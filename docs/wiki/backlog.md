# 未做

按改动面，不是日程。已落地的分层、Workspace、outbox 写入、密钥库、预先结构匹配、可选卷，见手册其它页。不要把下面这些当成「再改几个文件就齐了」。

## 表在、产品不在

- **Scene（场）**：`scenes` 表和领域类型有，编辑器与作品树都不出现。要做的话需要产品定义「场」和章、卷的关系，再补 Workspace / IPC / 树。
- **正史抽取**：`propose_canon_*`、`story-model::extract_mentions`、连续性检查仍可调用。UI 已不用抽取审核。顶栏「检查」仍入队连续性工具。
- **上下文三级预算**：ADR 0005 的后台检索、LLM 重排、钉住/忽略未做。现在只有本地多信号匹配。
- **插件按钮**：左下角有入口，没有完整插件管理界面。WASM 沙箱见下。

## 中等接线

- **TS / Rust 类型同源**：`types.ts` 与 `novel-domain` 手写两份。改字段容易漏一端。
- **两套匹配器**：桌面 Rust、浏览器 TypeScript，改信号要改两处。
- **feedback-memory** 已接到拒绝续写；没有单独的偏好管理界面（查看、停用、编辑）。
- **libraryApi 未收齐**：设置、续写、偏好仍有部分 `invoke` 散落在会话 hook 里。
- **未用的抽取前端**：`apps/client/src/canon/extract.ts` 仍在，界面不走它。

## 独立大件（不要当小重构）

- **WASM 插件沙箱**（[ADR 0004](../architecture/adr/0004-plugin-sandbox.md) 第三层）：`plugin.operation` 仍是内置执行器。
- **多设备同步**：outbox 已写，传输、冲突 UI、E2E 见 [sync-and-cloud.md](../sync-and-cloud.md) 阶段 2。
- **Android 伴侣**（[ADR 0006](../architecture/adr/0006-android-strategy.md)）：CI `android-build` 仍是占位。见 [android-companion.md](../android-companion.md)。
- **多人实时协作**：阶段更后，Yjs/CRDT。本地写作不能依赖云端在线。

## 文档漂移注意

旧 ADR / README 若仍写「正史审核闭环」，以 [0009](../architecture/adr/0009-canon-review-loop.md) 和 [产品](product.md) 为准。手册过时就改手册，不要另开一份 GitHub Wiki 各写各的。
