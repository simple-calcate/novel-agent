# 智能体：每个阶段看哪里

给**改本仓库的智能体**。作者看 [产品](product.md)；写插件看 [写插件](plugins.md)。契约签名以 [interfaces.md](../interfaces.md) 为准，分层禁区以 [layers.md](../architecture/layers.md) 为准。实现以代码为准。

不要把整本 wiki 一次读完。按下面五个阶段走：每阶段先回看仓库树，再只打开本阶段列出的文件。

## 开工前记住

- **结构**（`story_entries`，人物 / 设定 / 伏笔）是写作主路径。**正史**（canon 抽取、`canon_*` 表）库内仍在，界面不用。不要把抽取接到预选条。
- 用户点一下就能做 → `Workspace` → Tauri command → `libraryApi` → hook / 面板。不要先做成 Tool 再让 UI 调工具名。
- Agent / 队列 / 上下文浮带走 `kernel.call_tool`。作品库、结构、设置**不走**工具表。
- 前端作品库路径只通过 `apps/client/src/api.ts`。不要在树 / 结构 / 历史面板里直接 `invoke`。
- 浏览器预览（`pnpm --filter @novel-agent/client dev`）是**内存**；桌面（`pnpm tauri dev`）才是 SQLite + 密钥库。
- 产品阶段 1：本机 SQLite。同步传输、冲突 UI、Android APK、LLM 重排、正史抽取 UI、插件商店都还没做，不要当成本任务，除非任务明确要求。见 [未做](backlog.md) 与 [sync-and-cloud.md](../sync-and-cloud.md)。

## 现在仓库长什么样

```
墨枢/
  apps/client/                 React UI（Vite）
    src/                       界面、hooks、libraryApi、匹配的 TS 副本
    src-tauri/                 Tauri 宿主：装配内核 + JSON 翻译
      src/lib.rs               AppState、generate_handler!（命令表锁在这里）
      src/commands/            按领域拆 command，禁止写业务分支
  crates/
    domain/                    实体与事件，无 IO
    kernel/                    注册表、预算、工具分发、事件总线；无 SQLite / HTTP
    extensions/                内置扩展 + Workspace
    storage/                   迁移、单写者 StorageHandle、仓储
    automation/                信号、规则、队列状态机（真正执行在 queue.tick）
    context-hints/             段落 ↔ 结构条目匹配（桌面）
    context-engine/            ACP 风格装配（context.assemble），不是预选条
    feedback-memory/           拒绝续写后的偏好规则
    story-model/               启发式正史 / 连续性（非 UI 主路径）
    plugin-host/               清单、权限、桌面 wasmi（Android 忽略 WASM）
  packages/
    plugin-sdk/                MIT：definePlugin
    plugin-compile/            MIT：AssemblyScript → 无导入 WASM
    workflow-builder/          MIT：defineWorkflow 与模板
    event-schema/              MIT：事件信封
    match-fixtures/            宿主：匹配黄金用例（Rust + TS 共用）
    shared-types/              宿主：IPC 形状样例（两边反序列化）
  plugins/                     打包清单；hello-names 带 wasmBase64
  docs/wiki/                   现在怎么用、怎么改、还缺什么
  docs/interfaces.md           稳定契约
  docs/architecture/           分层禁区 + ADR
```

调用链（改任何用户可见能力时都沿这条走）：

```
UI  ──libraryApi──► Tauri command（只译 JSON）
                 ──► Workspace（作品库 / 结构 / 设置 / 续写）
                 ──► Kernel.call_tool / dispatch / run_continuation
                 ──► StorageHandle 单写者 ──► Repository（SQLite）
```

`StorageHandle` 同线程嵌套 `with` 返回 `Reentrancy`。禁止在持锁时 `kernel.dispatch`。Workspace 必须先写完再派事件。

## 五个阶段

### 阶段 1 · 定向（还没改文件）

| 任务像什么 | 先读 | 先不要读 |
|---|---|---|
| 改作者能看见的行为 | 本页树 + [产品](product.md) + [术语](glossary.md) | ADR 全文、插件 ABI |
| 改分层 / 写路径 / 新 crate | 本页树 + [架构](architecture.md) + [layers.md](../architecture/layers.md) | 产品界面细节 |
| 改 IPC / 字段 / 命令 | 本页「按层改」+ [interfaces.md](../interfaces.md) §6 检查表 | 写作协议样章 |
| 改匹配规则 | 本页「预选条」切片 + [产品](product.md)「当前段落预选」 | 正史 / story-model |
| 写插件 / WASM | [写插件](plugins.md) | Workspace CRUD |
| clone 后要跑起来 | [开发](development.md)「运行」 | backlog 大件 |
| 不确定词（正史、预选条、Workspace） | [术语](glossary.md) | — |

问自己三句，再进阶段 2：

1. 作者能不能看见？能 → 结束时改 [产品](product.md)。
2. 命令名或 JSON 字段变了吗？变了 → 必须改 interfaces §5，且与 `generate_handler!` 一致。
3. 这是阶段 1 已有能力，还是 [未做](backlog.md) 里的独立大件？大件不要当小重构。

### 阶段 2 · 定切片

对照下一节「切片 → 文件」，列出**准备动的路径**。同一切片里桌面和浏览器预览常常是两套实现（匹配、修订 diff、插件运行、作品库持久化）。改规则就要两边都改。

切片定错的典型信号：去改 `story-model` / `canon` 仓储，但产品要的是右侧「结构」或预选条。

### 阶段 3 · 按层改

用户点一下就能做的新能力，按这个顺序，不要跳：

1. `novel-domain` 类型（serde **camelCase**）
2. `crates/storage/migrations/00xx_*.sql`，并登记进 `migrations.rs` 的 `MIGRATIONS` 数组
3. `crates/storage/src/repository/<聚合>.rs` + 仓储测试
4. `Workspace` 方法（写完再 `dispatch`）
5. `apps/client/src-tauri/src/commands/<领域>.rs` 里的 command → `lib.rs` 的 `generate_handler!` → [interfaces.md](../interfaces.md) **§5 表格**
6. `apps/client/src/types.ts` + `packages/shared-types/examples.json`
7. `libraryApi`：**桌面 invoke 和内存实现都要写**
8. hook / 面板；界面文案不要进仓储
9. 产品能看见 → [产品](product.md)；新词 → [术语](glossary.md)

只换实现、不换签名：在扩展里 `register_tool` 或覆盖同名工具。宿主 command 里不要写业务分支。

Agent / 队列可调用的能力：实现 `Tool`，在扩展里注册；改工具 id 时同步工作流模板和 `OPERATION_LABELS`。

### 阶段 4 · 验证

跑本切片的测试，不要一上来 `cargo test --workspace` 代替定向。切片测过再按需加宽：

| 你改了 | 至少跑 |
|---|---|
| 领域类型 / 命令表 / examples.json | `cargo test -p novel-domain --test ipc_contract`；前端 `types.contract.test.ts` |
| 仓储 / 迁移 | `cargo test -p novel-storage` |
| Workspace / 续写编排 | `cargo test -p novel-extensions` |
| 宿主 command | `cargo test -p novel-agent-client`（`command_tests`） |
| 匹配信号 | `cargo test -p novel-context-hints` **和** `pnpm --filter @novel-agent/client test`；先改 `packages/match-fixtures/cases.json` |
| 修订对比 | Rust `novel-domain::diff_texts` + 前端 `editor/textDiff.ts` 两边用例 |
| 插件 SDK / 编译 / 工作流模板 | 对应 `pnpm --filter @novel-agent/<包> test` |
| UI 交互 | `pnpm --filter @novel-agent/client test` 与 `typecheck` |

叠放在功能分支上的 PR **不会**跑 `.github/workflows/ci.yml`（只对 `main`）。合并前仍应在本地跑 [开发](development.md) 里那一组。

浏览器预览验不了：SQLite、outbox、密钥链、真正的模型调用、桌面 `similar` 行内 diff。

### 阶段 5 · 改手册

| 变了什么 | 改哪 |
|---|---|
| 作者能看见的行为 | [产品](product.md)，必要时 [ADR 0009](../architecture/adr/0009-canon-review-loop.md) |
| 切层 / 写路径 | [架构](architecture.md)、[layers.md](../architecture/layers.md)、对应 ADR |
| 命令 / 仓储签名 | [interfaces.md](../interfaces.md)，不要只改 wiki |
| 新词 | [术语](glossary.md) |
| 本页树、切片、测试落点 | **本页** 和 [开发](development.md) |
| 还没做 | [未做](backlog.md)，不要写进产品页假装已有 |

手册、ADR、README 打架时：产品行为以 [产品](product.md) 和 0009 为准；「为什么」以 ADR 为准；签名以 interfaces 为准。

## 切片 → 文件

路径从仓库根算。先打开「从这里开始」，需要签名时再打开 interfaces。

| 切片 | 从这里开始 | 还要动 | 测试 |
|---|---|---|---|
| **作品库** 书/卷/章/场树 | `crates/domain/src/content.rs`；`crates/storage/src/repository/library.rs` | `crates/extensions/src/workspace.rs`；`apps/client/src-tauri/src/commands/library.rs`；`apps/client/src/hooks/useLibrary.ts`；`apps/client/src/components/LibraryActions.tsx`；`apps/client/src/App.tsx` | `cargo test -p novel-storage --test repository_test`；`cargo test -p novel-extensions --test workspace_test`；`apps/client/src/api.test.ts` |
| **结构条目** | `crates/domain/src/story.rs`（`StoryEntry`）；`crates/storage/src/repository/structure.rs` | Workspace `*_story_entry`；`apps/client/src-tauri/src/commands/canon.rs`（结构命令也在这）；`apps/client/src/hooks/useStructure.ts`；`apps/client/src/components/StructurePanel.tsx` | `cargo test -p novel-storage --test structure_test`；`workspace_test` |
| **预选条 / 匹配** | `crates/context-hints/`；`apps/client/src/structure/match.ts`；`packages/match-fixtures/cases.json` | `crates/extensions/src/hints.rs`（工具 `context.hints`）；`apps/client/src-tauri/src/commands/editor.rs` `context_hints`；`apps/client/src/hooks/useEditorSession.ts`；`apps/client/src/components/ContextRail.tsx` | `cargo test -p novel-context-hints`；`apps/client/src/structure/match.test.ts`。改信号必须两边过共享 fixtures |
| **修订历史** | `crates/domain/src/diff.rs`；`crates/storage/src/repository/revisions.rs` | Workspace `list/diff/restore_chapter_revision`；`apps/client/src-tauri/src/commands/library.rs`；`apps/client/src/components/HistoryPanel.tsx`；浏览器预览另写 `apps/client/src/editor/textDiff.ts` | `cargo test -p novel-storage --test commit_patch_test`；`apps/client/src/editor/textDiff.test.ts` |
| **写作协议 / 块** | `crates/domain/src/protocol.rs`；`apps/client/src/editor/protocol.ts` | `apps/client/src/editor/ModeSwitch.ts`；`ThinkingBlock.tsx`；`crates/extensions/src/blocks.rs`；[writing-protocol.md](../writing-protocol.md) | `apps/client/src/editor/protocol.test.ts` 等；`cargo test -p novel-extensions --test block_mode_event_test` |
| **示例章** | `apps/client/src/editor/examples/fog-harbor.json` | `apps/client/src/editor/sampleChapter.ts`（只按 JSON 写入，不写死人名）；[examples/fog-harbor.md](../examples/fog-harbor.md) | `apps/client/src/editor/sampleChapter.test.ts`。结构只在**新建**示例章时写入 |
| **续写 / 密钥 / 偏好** | `crates/extensions/src/secrets.rs`；`crates/feedback-memory/` | `apps/client/src-tauri/src/commands/settings.rs`；`apps/client/src-tauri/src/commands/editor.rs` `generate_continuation`；`apps/client/src/components/SettingsModal.tsx`；`PreferencePanel.tsx` | `cargo test -p novel-feedback-memory`；`workspace_test` / `provider_http_test` |
| **队列 / 工作流** | `crates/automation/`；`crates/extensions/src/queue.rs`、`workflow.rs` | `packages/workflow-builder/`；`apps/client/src-tauri/src/commands/queue.rs`；`apps/client/src/hooks/useQueue.ts`；`apps/client/src/components/WorkflowPanel.tsx`；`apps/client/src/workflow/labels.ts` | `cargo test -p novel-automation`；`queue_flow_test`、`manual_enqueue_test`；`pnpm --filter @novel-agent/workflow-builder test` |
| **插件** | `packages/plugin-sdk/`；`crates/plugin-host/` | `packages/plugin-compile/`；`plugins/*/plugin.json`；`crates/extensions/src/plugins.rs`；`apps/client/src-tauri/src/commands/plugins.rs`；`apps/client/src/components/PluginModal.tsx`；`apps/client/src/plugins/format.ts` | `cargo test -p novel-plugin-host`；plugin-sdk / plugin-compile 包测试。改 guest 后 `pnpm --filter @novel-agent/plugin-compile compile:hello-names` |
| **上下文装配** | `crates/context-engine/` | `crates/extensions/src/context.rs`（`context.assemble`）；`apps/client/src-tauri/src/commands/editor.rs` `build_context_package` | `cargo test -p novel-context-engine`。这不是预选条 |
| **正史（非主路径）** | `crates/story-model/`；`crates/storage/src/repository/canon.rs` | Workspace `propose_canon_*`；`apps/client/src-tauri/src/commands/canon.rs`；`apps/client/src/canon/extract.ts` 只给浏览器内存测试 | `cargo test -p novel-story-model`；`canon_test`。**不要**接到 `ContextRail` |
| **outbox** | `crates/storage/src/repository/outbox.rs` | 写路径走 `write_with_outbox`；`apps/client/src-tauri/src/commands/sync.rs`；工作流页「写出 journal」 | `cargo test -p novel-storage --test outbox_test`。写出 JSONL **不是**设备间同步 |
| **内核装配** | `crates/kernel/`；`crates/extensions/src/lib.rs`（`BuiltinsExtension`） | `apps/client/src-tauri/src/lib.rs` `run()`：注入 `StorageHandle` + `SecretVault` | `cargo test -p novel-kernel` |
| **契约 / 类型同源** | [interfaces.md](../interfaces.md)；`packages/shared-types/examples.json` | `apps/client/src/types.ts`；`crates/domain/tests/ipc_contract.rs` | `cargo test -p novel-domain --test ipc_contract`；`apps/client/src/types.contract.test.ts`。没有代码生成，靠两边反序列化 |

## 宿主命令落在哪个文件

`generate_handler!` 在 `apps/client/src-tauri/src/lib.rs`。实现按领域拆在 `commands/`。新增命令：写模块 → 登记 handler → 写 interfaces §5。漏登记 `ipc_contract` 会红。

| 模块 | 命令（约） |
|---|---|
| `apps/client/src-tauri/src/commands/library.rs` | 作品库 CRUD、场次、`load/save_chapter`、修订 list/diff/restore |
| `apps/client/src-tauri/src/commands/editor.rs` | `editor_tick`、`context_hints`、`generate_continuation`、批注、领域事件、块模式、`build_context_package` |
| `apps/client/src-tauri/src/commands/settings.rs` | `save/load_model_config`、偏好 list/set/record |
| `apps/client/src-tauri/src/commands/plugins.rs` | `list_plugins`、`run_plugin_operation`、`install_plugin_manifest` |
| `apps/client/src-tauri/src/commands/queue.rs` | `enqueue_job`、`run_queue_step`、`list_jobs`、`kernel_tools` |
| `apps/client/src-tauri/src/commands/sync.rs` | `pending_outbox_count`、`flush_outbox_journal` |
| `apps/client/src-tauri/src/commands/canon.rs` | `propose/list/review_canon_*` **以及** `create/list/delete_story_entry` |

编辑器会话（心跳、模式切换、装配上下文）可以 `invoke`；其余前端走 `libraryApi`。

## 领域类型落在哪个文件

`crates/domain/src/`，无 IO。

| 文件 | 放什么 |
|---|---|
| `content.rs` | `Project` / `Book` / `Volume` / `Chapter` / `Scene`、`LibrarySnapshot`、`ChapterBody`、块 |
| `story.rs` | `StoryEntry`（主路径）与 `CanonEntity` / `CanonFact` / `CanonProposal`（非主路径） |
| `diff.rs` | `similar` 正文对比 |
| `protocol.rs` | 写作协议（思考 / 正文 / 拍） |
| `events.rs` | `DomainEvent`、`EventKind` |
| `jobs.rs` | 队列任务视图 |
| `patches.rs` | `ContentPatch`、`PreferenceRule` |
| `plugins.rs` | `PluginSummary` / `PluginResult` |
| `ids.rs` | 各类 Id |
| `work.rs` | `WorkContextRef`、上下文包（给 `context.assemble`） |
| `actor.rs` | 操作者 |

JSON 字段 camelCase。改字段时同步迁移、`types.ts`、`examples.json`。

## 扩展与仓储

`crates/extensions/src/`：`workspace.rs` 是应用门面；`lib.rs` 里 `BuiltinsExtension` 依次注册 providers、core_tools、blocks、workflow、queue、hints、context assembly、plugin-host。`secrets.rs` 是 `SecretVault`，不要把密钥写进 `save_setting`。

`crates/storage/src/repository/`：

| 模块 | 职责 |
|---|---|
| `library` | 作品 / 书 / 卷 / 章 / 场 |
| `revisions` | 修订快照、补丁、块序列；`list_revisions` / `diff_revisions` |
| `structure` | `story_entries` |
| `canon` | 启发式正史候选 |
| `queue` / `automation` | 任务与工作流 |
| `outbox` | 与写操作同一事务入队 |
| `feedback` | 偏好规则、纠正记录 |

迁移是 `crates/storage/migrations/0001_init.sql` … `0010_feedback.sql`，必须按序号加进 `migrations.rs`。

## 前端从哪进

| 路径 | 职责 |
|---|---|
| `src/api.ts` | `libraryApi` + `isTauriRuntime()`；内存实现与桌面分流 |
| `src/types.ts` | 与 domain camelCase 对齐 |
| `hooks/useLibrary.ts` | 作品树、当前书/卷/章/场 |
| `hooks/useStructure.ts` | 预先结构 |
| `hooks/useEditorSession.ts` | 正文、预选、续写、模型配置、偏好、恢复修订后重挂 |
| `hooks/useQueue.ts` | 任务队列 |
| `components/ContextRail.tsx` | 预选条（钉住 / 忽略） |
| `components/SceneStrip.tsx` | 本章场次 |
| `components/StructurePanel.tsx` | 右侧结构 |
| `components/HistoryPanel.tsx` | 修订 |
| `components/PreferencePanel.tsx` | Agent 页偏好 |
| `components/WorkflowPanel.tsx` | 模板、队列、outbox journal |
| `components/PluginModal.tsx` | 打包插件 |
| `components/SettingsModal.tsx` | 模型与密钥 |
| `editor/examples/fog-harbor.json` | 示例章正文 + `story` |
| `structure/match.ts` | 浏览器侧匹配器 |
| `canon/extract.ts` | 仅内存 `proposeCanon` 测试，不是产品路径 |

## 产品阶段（别超前做）

| 阶段 | 现在有没有 | 文档 |
|---|---|---|
| 1 本机 SQLite + 可选 JSONL journal | 有 | [sync-and-cloud.md](../sync-and-cloud.md) |
| 2 设备间同步、冲突 UI、E2E | 没有 | 同上 |
| 3 订阅控制面、插件签名商店 | 没有 | 同上；作者保证见 [trust.md](trust.md) |
| Android 伴侣 APK | 没有；CI 只对无 C 依赖 crate `aarch64-linux-android` check | [android-companion.md](../android-companion.md) |
| 预选条 LLM 重排 | 没有；本地规则 + 词汇检索已有 | [ADR 0005](../architecture/adr/0005-context-hints.md) |

## 常见走错

- 把「从正文抽人物再审核」当主路径。已否，见 [ADR 0009](../architecture/adr/0009-canon-review-loop.md)。
- 结构命令写在 `commands/canon.rs` 里，并不表示结构和正史是一张表。
- 在 `StorageHandle::with` 闭包里 `dispatch`。
- 密钥进 SQLite、outbox 载荷或导出文件。outbox 只带 id / 修订号。
- 只改桌面或只改浏览器预览的匹配 / diff / `libraryApi` 内存分支。
- 改了 `generate_handler!` 却忘了 interfaces §5（或反过来）。
- 打开示例章时把结构写死在 TS 常量里。改 `fog-harbor.json` 的 `story`。
- 另开一份 GitHub Wiki。手册就在 `docs/wiki/`，跟代码一起改。
