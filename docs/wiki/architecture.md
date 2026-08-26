# 架构：现在怎么切

分层总图和禁区见 [layers.md](../architecture/layers.md)。契约签名见 [interfaces.md](../interfaces.md)。这里只补当前实现里容易走错的几条。

## 调用链

```
UI  ──libraryApi──► Tauri command（只译 JSON）
                 ──► Workspace（作品库 / 结构 / 设置 / 续写）
                 ──► Kernel.call_tool / dispatch / run_continuation
                 ──► StorageHandle 单写者 ──► Repository（SQLite）
```

作品库、结构条目、设置**不走**工具表。Agent、队列、上下文浮带走 `call_tool`。

`StorageHandle` 同线程嵌套 `with` / `execute` 返回 `Reentrancy`，禁止在持锁时 `kernel.dispatch`。Workspace 必须先写完再派事件。见 [ADR 0008](../architecture/adr/0008-workspace-storage-handle.md)。

宿主（`apps/client/src-tauri`）只允许：解析 camelCase JSON、调 `Workspace` 或内核、封 `{ ok, data, error }`。不要在 command 里写业务分支。

## 仓库地图

| 路径 | 职责 |
|---|---|
| `apps/client` | React UI；浏览器预览走内存 `libraryApi` |
| `apps/client/src-tauri` | Tauri 宿主，JSON 翻译 |
| `crates/kernel` | 注册表、预算、工具分发、事件总线；无 SQLite / HTTP |
| `crates/extensions` | 内置扩展 + `Workspace` |
| `crates/domain` | 实体与事件，无 IO |
| `crates/storage` | 迁移、单写者、仓储模块 |
| `crates/context-hints` | 段落 ↔ 结构条目匹配 |
| `crates/feedback-memory` | 拒绝续写后的偏好规则 |
| `crates/story-model` | 启发式正史 / 连续性（非 UI 主路径） |
| `crates/automation` | 信号、规则、队列状态机 |
| `crates/plugin-host` | 清单、权限、桌面 wasmi 沙箱（Android 走内置） |
| `packages/` | 事件 schema、插件 SDK、工作流模板 |

## 仓储切分

`crates/storage/src/repository/`：

| 模块 | 职责 |
|---|---|
| `library` | 作品 / 书 / 卷 / 章 / 场 |
| `revisions` | 修订、补丁、块序列 |
| `structure` | `story_entries` 预先结构 |
| `canon` | 启发式正史候选（非 UI 主路径） |
| `queue` / `automation` | 任务与工作流 |
| `outbox` | 与写操作同一事务入队 |
| `feedback` | 偏好规则、纠正记录 |

`scenes` 表已接到 Workspace / IPC / 编辑器「本章场次」。删场不删章正文。POV 可选，指向人物结构条目。

## 写路径与 outbox

业务状态、operation log、任务和 outbox 在同一 SQLite 事务提交。[ADR 0002](../architecture/adr/0002-revision-and-outbox.md) 要求如此；同步发送仍是 [阶段 2](../sync-and-cloud.md)。载荷只带 id / 修订号，不带正文、不带 API Key。

`list_pending_outbox` / `mark_outbox_delivered` / `count_pending_outbox` 给同步消费者。`Workspace::flush_outbox_journal` 把待发送行追加写成 JSONL 再标记 delivered。这是本机写出，不是设备间传输。冲突 UI 与 E2E 仍是 [阶段 2](../sync-and-cloud.md)。

## 结构匹配

- 产品路径：`story_entries` + `HintEngine::rank_entries`（[ADR 0009](../architecture/adr/0009-canon-review-loop.md)）
- 桌面：`context.hints` 工具，入参含 `nearbyText` 与可选 `lookbackText`
- 浏览器：`apps/client/src/structure/match.ts`（`isTauriRuntime()` 为假时）
- 两边共用 `packages/match-fixtures/cases.json`；改信号必须两边测试都过
- 预选条可钉住 / 忽略（本机 `localStorage`）
- 第二级：本地词汇检索（当前段 token → 条目标题/说明）；`index.rebuild` 会把 `story_entries` 写入 FTS
- LLM 重排未做
- `story-model` 的抽取与连续性检查仍可被工具调用，**不是**编辑器预选条的数据源
- `canon` 仓储与 `story_entries` 不要混用一张表、一条 API
- 写作协议（思考/正文/拍）见 [ADR 0011](../architecture/adr/0011-writing-protocol.md)；导出走 `training.export`，思考里的 `@` 是标签不是正史行

[ADR 0005](../architecture/adr/0005-context-hints.md) 的三级预算：本地多信号 → 本机词汇检索 → 可选 LLM 重排。前两级已落地；LLM 重排未做。

## 密钥

`SecretVault` 与 `StorageHandle` 一并注入内核。[ADR 0010](../architecture/adr/0010-secret-vault.md)：

- OS 密钥链优先（service `com.moshu.novel-agent`）
- 失败则应用数据目录 `secrets/`，文件权限 0600
- 测试用 `SecretVault::memory()`
- `model_config` 只存 provider / baseUrl / model
- 读给 UI 的配置带 `apiKeySet`，`apiKey` 恒为空串；保存时空密钥表示保持
- 旧库 JSON 里的 `apiKey` 在读取时迁出并改写设置

不要把密钥写进 `save_setting`、outbox 载荷或导出文件。

## 内核装配

```rust
let kernel = Kernel::builder()
    .service(storage)   // Arc<StorageHandle>
    .service(secrets)   // Arc<SecretVault>
    .extension(BuiltinsExtension)?
    .build()?;
let workspace = Workspace::new(&kernel);
```

同名 Tool / Provider 后注册覆盖前者。新增「用户点一下就能做」的能力走 Workspace + IPC，不要先做成 Tool 再让 UI 去调工具名。
