# 开发

## 运行

```bash
pnpm install
cargo check --workspace
pnpm --filter @novel-agent/client dev          # 浏览器预览，内存作品库
cd apps/client && pnpm tauri dev               # 桌面，SQLite + 密钥库
```

测试：

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
pnpm --filter @novel-agent/client test
pnpm --filter @novel-agent/client typecheck
pnpm --filter @novel-agent/plugin-compile test
pnpm --filter @novel-agent/plugin-sdk test
```

CI（`.github/workflows/ci.yml`）只在面向 `main` 的 pull request / push 上跑。叠放在功能分支上的 PR 不会触发这套 workflow；合并前仍应在本地跑上面几条。`frontend-check` 会跑 typecheck、build 和前端 test。

`ipc_contract` 会核对 [interfaces.md](../interfaces.md) §5 命令表与 `generate_handler!` 是否一致。漏登记命令会红。

## 浏览器 vs 桌面

`isTauriRuntime()` 为假时，`libraryApi` 用内存实现，匹配走 TypeScript。能验 UI、树、预选条、场次、偏好、修订历史面板。不能验 SQLite、outbox、密钥链、真正的模型调用，也不能验桌面用的 `similar` 行内 diff（预览用 `editor/textDiff.ts` 的行级 LCS）。

改作品库或结构 CRUD 时：浏览器测交互，桌面或 `cargo test` 测持久化。改匹配规则时两边都要测：`crates/context-hints` 与 `apps/client/src/structure/match.test.ts`，并更新 `packages/match-fixtures/cases.json`。改修订对比时：Rust `novel-domain::diff_texts` 与前端 `textDiff.ts` 都要有用例；IPC 形状以 `RevisionDiff` 为准。

## 改能力时的落点

与 [layers.md](../architecture/layers.md) 一致：

| 你要改的 | 动哪里 |
|---|---|
| 数据形状 | `novel-domain` + `crates/storage/migrations` + [interfaces.md](../interfaces.md) + `apps/client/src/types.ts` + `packages/shared-types/examples.json` |
| 用户点一下就能做 | `Workspace` → Tauri command → `libraryApi` → 对应 UI hook / 面板 |
| 章节修订历史 / 对比 | `domain/diff.rs`（`similar`）→ `storage` `list_revisions` / `diff_revisions` → Workspace / IPC → `HistoryPanel`；浏览器预览另写 `editor/textDiff.ts` |
| Agent / 队列可调用 | `Tool` + `register_tool`；工作流模板与 `OPERATION_LABELS` |
| 只换实现 | `Kernel::builder().extension(...)` 或覆盖同名工具 |
| 段落匹配规则 | `crates/context-hints` **和** `apps/client/src/structure/match.ts`，加上共享 fixtures |
| 结构条目增删改 | `storage/structure` → Workspace → `update_story_entry` → `libraryApi` → `StructurePanel` |
| 插件清单 / 工作流定义 | MIT 包 `packages/plugin-sdk`、`packages/workflow-builder`、`packages/plugin-compile` |
| 模型密钥 | `SecretVault`，不要写进 `save_setting` |
| 界面文案 / 树交互 | `apps/client/src/App.tsx` 与 `components/`，不改仓储 |

前端作品库路径只通过 `apps/client/src/api.ts` 的 `libraryApi`。不要在作品树 / 结构 / 历史面板里直接 `invoke`。编辑器会话（心跳、模式切换）仍可 `invoke` 对应命令。

## 前端模块

| 文件 | 职责 |
|---|---|
| `hooks/useLibrary.ts` | 作品树、增删改、当前书/卷/章/场 |
| `hooks/useStructure.ts` | 预先结构列表 |
| `hooks/useEditorSession.ts` | 正文、预选、续写、模型配置、偏好、恢复修订后重挂编辑器 |
| `hooks/useQueue.ts` | 任务队列 |
| `components/ContextRail.tsx` | 编辑器上方预选条（钉住/忽略） |
| `components/SceneStrip.tsx` | 本章场次 |
| `components/StructurePanel.tsx` | 右侧结构：添加、改名称/别名/说明、确认删除 |
| `components/PreferencePanel.tsx` | Agent 页偏好 |
| `components/PluginModal.tsx` | 打包插件列表 |
| `components/WorkflowPanel.tsx` | 工作流模板、队列、outbox journal |
| `components/HistoryPanel.tsx` | 修订列表、与上一版对比、恢复 |
| `editor/textDiff.ts` | 浏览器预览的行级对比（桌面走 Rust `similar`） |
| `editor/sampleChapter.ts` | 把 `editor/examples/*.json` 装进作品库；林默等人写在 JSON 的 `story` 里。只在新建示例章时写入结构，已有章节再打开不会复活作者删掉的条目 |
| `structure/match.ts` | 浏览器侧匹配器 |

## 改接口检查表

改稳定契约时对照 [interfaces.md](../interfaces.md) 末尾清单：

1. domain 字段 → serde camelCase、SQLite 迁移、TS `types.ts`、`packages/shared-types/examples.json`
2. command 名或字段 → `libraryApi`（若前端要调）、宿主 command 测试、interfaces **§5 表格**（与 `generate_handler!` 对齐）
3. 工具 id → 工作流模板、`OPERATION_LABELS`、interfaces 工具表
4. 产品能看见的行为 → [product.md](product.md)；新词 → [glossary.md](glossary.md)；本页落点表 / 前端模块表
5. `cargo test --workspace` 与前端 test / typecheck

## 文档

- 产品行为变了 → 改 [product.md](product.md)，必要时改 [ADR 0009](../architecture/adr/0009-canon-review-loop.md)
- 切层方式变了 → 改 [architecture.md](architecture.md)、[layers.md](../architecture/layers.md)、对应 ADR
- 命令 / 仓储签名变了 → 改 [interfaces.md](../interfaces.md)，不要只改 wiki
- 作者能叫出的新词 → 改 [glossary.md](glossary.md)
- 新能力还没做 → 写进 [backlog.md](backlog.md)，不要写进产品页假装已有
- 手册、ADR、README 打架时：产品行为以 [product.md](product.md) 和 [0009](../architecture/adr/0009-canon-review-loop.md) 为准；「为什么」以 ADR 为准；签名以 interfaces 为准
