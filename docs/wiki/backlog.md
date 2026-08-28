# 未做

按改动面，不是日程。场次、偏好界面、插件列表、libraryApi 收齐、匹配黄金用例、IPC 样例，见手册其它页。下面这些不要当成「再改几个文件就齐了」。

## 表在、产品不在（已收口的）

- **Scene（场）**：已接到编辑器「本章场次」。
- **偏好**：Agent 页可查看、停用。
- **插件按钮**：列出打包清单。「人名点名」可对当前正文运行；其它打包项仍是内置占位。
- **libraryApi**：设置、续写、偏好、插件、场次都走 `libraryApi`。
- **匹配双端**：共享 `packages/match-fixtures/cases.json`。实现仍是两套，改规则改两处并跑共享用例。
- **类型同源**：没有代码生成。`packages/shared-types/examples.json` 由 Rust 与 TypeScript 两边反序列化，防止字段漂移。

## 仍未做的产品细节

- **正史抽取 UI**：API 仍在，界面不用。顶栏「检查」仍入队连续性工具。
- **上下文三级预算**：钉住/忽略已做。第二级是本机词汇检索（当前段的词去条目标题/说明里找，命中原因写「检索到…」）。**LLM 重排未做**。
- `apps/client/src/canon/extract.ts` 只给浏览器内存 `proposeCanon` 测试用。

## 独立大件（不要当小重构）

- **WASM 插件沙箱**（[ADR 0004](../architecture/adr/0004-plugin-sandbox.md) 第三层）：桌面 `plugin.operation` 在清单带 `wasmBase64` 时于 wasmi 运行。`definePlugin` 产出清单；`@novel-agent/plugin-compile` 用 AssemblyScript 把 guest 编成无导入 WASM。签名与商店没有。
- **工作流可视化编辑器**：`defineWorkflow` 与模板在 MIT 包里；应用里工作流页列出这些模板并可入队动作，没有画布编辑器。
- **多设备同步**：outbox 可写出本机 JSONL，**没有**传输、冲突 UI、E2E。见 [sync-and-cloud.md](../sync-and-cloud.md) 阶段 2。
- **Android 伴侣**（[ADR 0006](../architecture/adr/0006-android-strategy.md)）：CI 对无 C 依赖 crate 做 `aarch64-linux-android` 的 `cargo check`。没有 APK / NDK 全套。见 [android-companion.md](../android-companion.md)。
- **多人实时协作**：阶段更后，Yjs/CRDT。本地写作不能依赖云端在线。

## 文档漂移注意

旧 ADR / README 若仍写「正史审核闭环」，以 [0009](../architecture/adr/0009-canon-review-loop.md) 和 [产品](product.md) 为准。手册过时就改手册，不要另开一份 GitHub Wiki 各写各的。
