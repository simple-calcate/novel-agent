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
```

CI（`.github/workflows/ci.yml`）只在面向 `main` 的 pull request / push 上跑。叠放在功能分支上的 PR 不会触发这套 workflow；合并前仍应在本地跑上面几条。

## 浏览器 vs 桌面

`isTauriRuntime()` 为假时，`libraryApi` 用内存实现，匹配走 TypeScript。能验 UI、树、预选条、场次、偏好，不能验 SQLite、outbox、密钥链、真正的模型调用。

改作品库或结构 CRUD 时：浏览器测交互，桌面或 `cargo test` 测持久化。改匹配规则时两边都要测：`crates/context-hints` 与 `apps/client/src/structure/match.test.ts`，并更新 `packages/match-fixtures/cases.json`。

## 改能力时的落点

与 [layers.md](../architecture/layers.md) 一致：

| 你要改的 | 动哪里 |
|---|---|
| 数据形状 | `novel-domain` + `crates/storage/migrations` + [interfaces.md](../interfaces.md) + `apps/client/src/types.ts` + `packages/shared-types/examples.json` |
| 用户点一下就能做 | `Workspace` → Tauri command → `libraryApi` → 对应 UI hook |
| Agent / 队列可调用 | `Tool` + `register_tool`；工作流模板与 `OPERATION_LABELS` |
| 只换实现 | `Kernel::builder().extension(...)` 或覆盖同名工具 |
| 段落匹配规则 | `crates/context-hints` **和** `apps/client/src/structure/match.ts`，加上共享 fixtures |
| 模型密钥 | `SecretVault`，不要写进 `save_setting` |
| 界面文案 / 树交互 | `apps/client/src/App.tsx` 与 `components/`，不改仓储 |

前端只通过 `apps/client/src/api.ts` 的 `libraryApi`。不要在组件里直接 `invoke`。

## 前端模块

| 文件 | 职责 |
|---|---|
| `hooks/useLibrary.ts` | 作品树、增删改、当前书/卷/章/场 |
| `hooks/useStructure.ts` | 预先结构列表 |
| `hooks/useEditorSession.ts` | 正文、预选、续写、模型配置、偏好 |
| `hooks/useQueue.ts` | 任务队列 |
| `components/ContextRail.tsx` | 编辑器上方预选条（钉住/忽略） |
| `components/SceneStrip.tsx` | 本章场次 |
| `components/StructurePanel.tsx` | 右侧结构 |
| `components/PreferencePanel.tsx` | Agent 页偏好 |
| `components/PluginModal.tsx` | 打包插件列表 |
| `components/WorkflowPanel.tsx` | 工作流模板、队列、outbox journal |
| `structure/match.ts` | 浏览器侧匹配器 |

## 改接口检查表

改稳定契约时对照 [interfaces.md](../interfaces.md) 末尾清单：

1. domain 字段 → serde camelCase、SQLite 迁移、TS `types.ts`、`packages/shared-types/examples.json`
2. command 名或字段 → `libraryApi`、宿主 command 测试、interfaces 表格
3. 工具 id → 工作流模板、`OPERATION_LABELS`、interfaces 工具表
4. `cargo test --workspace` 与前端 test / typecheck

## 文档

- 产品行为变了 → 改 [product.md](product.md)，必要时改 [ADR 0009](../architecture/adr/0009-canon-review-loop.md)
- 切层方式变了 → 改 [architecture.md](architecture.md)、[layers.md](../architecture/layers.md)、对应 ADR
- 命令 / 仓储签名变了 → 改 [interfaces.md](../interfaces.md)，不要只改 wiki
- 新能力还没做 → 写进 [backlog.md](backlog.md)，不要写进产品页假装已有
