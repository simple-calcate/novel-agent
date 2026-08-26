# 接口清单

分层职责见 [architecture/layers.md](architecture/layers.md)。本文件列出**当前稳定契约**：改这些签名时请同步测试与本页。

## 1. 领域：作品层级

```
Project（作品） 1—n Book（书） 1—n 可选 Volume（卷） 1—n Chapter（章） 1—n 可选 Scene（场）
```

| 类型 | 含义 | 关键字段 |
|---|---|---|
| `Project` | 一部作品的工作区 | `id`, `title`, `createdAt`, `updatedAt` |
| `Book` | 一本书 | `id`, `projectId`, `title`, `synopsis`, `position` |
| `Volume` | 书下的可选卷 | `id`, `bookId`, `title`, `position` |
| `Chapter` | 可修订的正文单位 | `id`, `bookId`, `volumeId?`, `title`, `position`, `currentRevision`, `status` |
| `Scene` | 章内大纲场次，不替代正文 | `id`, `chapterId`, `title`, `position`, `povEntryId?` |

创建书时 `position = 0` 表示自动排到该作品末尾；章节同理。书必须属于已存在的作品，章必须属于该作品下的书，否则仓储返回 `NotFound`。

领域事件（`EventKind::as_str`）：

| 事件 | 何时发出 |
|---|---|
| `project.created` / `project.renamed` / `project.deleted` | 作品增删改 |
| `book.created` / `book.renamed` / `book.deleted` / `book.reordered` | 书 |
| `volume.created` / `volume.renamed` / `volume.deleted` / `volume.reordered` | 卷 |
| `chapter.created` / `chapter.renamed` / `chapter.deleted` / `chapter.reordered` | 章 |
| `scene.created` / `scene.renamed` / `scene.deleted` / `scene.reordered` | 场 |
| `canon.proposed` / `canon.accepted` / `canon.rejected` | 正史候选生成与作者审核 |
| `block.mode.changed` | 编辑器思考/正文切换 |
| `agent.finished` | 内核续写结束（可选） |

载荷 camelCase JSON。完整信封见 `packages/event-schema`。

## 2. 内核：`novel-kernel`

宿主只应调用：

| 方法 | 作用 |
|---|---|
| `Kernel::builder()` | 注入 `service`、`extension` / `tool` / `provider_factory` / `subscriber` |
| `call_tool(id, input)` | 执行已注册工具 |
| `dispatch(event)` | 按 `event_type` 通知订阅者 |
| `run_continuation(config, spec)` | 流式续写，预算硬截断 |
| `provider(config)` | 按名字新建 Provider |
| `service::<T>()` | 取注入服务（如 `StorageHandle`） |
| `tool_registry().describe()` | 自描述：`id` + `summary` + `input_schema` |

扩展实现 `Extension::setup(&mut KernelBuilder)`。同名 `Tool` / Provider 后注册覆盖前者。

`Tool` 契约：`id` 全局唯一；通过 `ToolContext` 调其它工具或 `service`，不要依赖 Tauri。

## 3. 内置工具（`BuiltinsExtension`）

以 `kernel_tools` 命令或 `tool_registry().ids()` 为准。常见 id：

| id | 职责 |
|---|---|
| `document.save` | 登记保存（正文走 `save_chapter`） |
| `index.rebuild` | 重建 FTS |
| `continuity.check` | 正史连续性 |
| `backup.create` | 备份 |
| `agent.continuation` / `agent.run` | Agent |
| `queue.tick` | 领取并执行一个队列任务 |
| `context.hints` / `context.assemble` | 浮带与上下文包 |
| `block.save` / `block.edit` / `training.export` | 块模型与按写作协议导出训练数据 |
| `plugin.install` / `plugin.operation` | 插件 |

## 4. 仓储：`StorageHandle` + `Repository`

作品库：

- `create_project(title)`
- `create_book(project_id, title, synopsis, position)`
- `create_chapter(project_id, book_id, title, position)` / `create_chapter_with_volume(..., volume_id)`
- `create_scene(project_id, chapter_id, title, position, pov_entry_id?)`
- `list_projects` / `list_books` / `list_volumes` / `list_chapters` / `list_scenes`
- `rename_project` / `delete_project`
- `rename_book` / `delete_book` / `move_book(delta)`
- `create_volume` / `rename_volume` / `delete_volume` / `move_volume(delta)`
- `rename_scene` / `set_scene_pov` / `delete_scene` / `move_scene(delta)`
- `rename_chapter` / `delete_chapter` / `move_chapter(delta)`
- `save_chapter_snapshot` / `save_block_sequence` / `block_sequence`
- `chapter_text` / `current_revision` / `commit_patch`
- `propose_canon_mentions` / `list_canon_proposals` / `set_fact_status`
- `create_story_entry` / `list_story_entries` / `delete_story_entry`
- `list_canon_entities_for_project` / `list_canon_facts_for_project` / `list_plot_threads_for_project`

`Repository` 按聚合拆在 `crates/storage/src/repository/`：`library`、`revisions`、`canon`、`structure`、`queue`、`automation`。

设置：`save_setting` / `get_setting`；当前作品键 `SETTING_ACTIVE_PROJECT`。
模型配置：`Workspace::save_model_config` / `load_model_config`；API Key 走 `SecretVault`，不进 `app_settings`。

Outbox：作品库 / 修订 / 入队 / 结构写路径在同一事务插入 `outbox`；`list_pending_outbox` / `mark_outbox_delivered` / `count_pending_outbox`。`Workspace::flush_outbox_journal(path)` 把待发送行追加写成 JSONL 再标记 delivered。这是本机写出，不是设备间同步。

偏好：`save_preference_rule` / `list_preference_rules` / `set_preference_status` / `save_correction`；拒绝续写后写入，下次 `generate_continuation` 拼进 system prompt。停用的规则不进提示。

单写者：宿主注入 `Arc<StorageHandle>`。所有 SQLite 访问走 `StorageHandle::with` /
`execute`。**禁止在 `with` 闭包内 `kernel.dispatch`**：同线程嵌套访问返回
`StorageError::Reentrancy`，而不是死锁。应用层 `Workspace` 保证先写完再派发事件。

## 4b. 应用层：`Workspace`

宿主只应通过 `Workspace::new(&kernel)` 做作品库、设置、手动入队和续写配置解析：

- `create_project` / `create_book` / `create_volume` / `create_chapter` / `create_scene`
- `load_library` / `set_active_project` / `load_chapter` / `save_chapter`
- `rename_*` / `delete_*` / `move_*` / `set_scene_pov`
- `enqueue_job` / `list_jobs` / `save_setting` / `get_setting`
- `generate_continuation`
- `save_model_config` / `load_model_config`
- `record_generation_feedback` / `list_preference_rules` / `set_preference_status`
- `list_plugins`
- `pending_outbox_count` / `flush_outbox_journal`
- `propose_canon_from_chapter` / `list_canon` / `review_canon_fact`
- `create_story_entry` / `list_story_entries` / `delete_story_entry`

`LibrarySnapshot`、`ChapterBody`、`JobView`、`CanonProposal`、`StoryEntry`、`Scene`、`PreferenceRule`、`PluginSummary` 定义在 `novel-domain`。

产品路径：作者预先添加人物 / 设定 / 伏笔；`context.hints` 按当前段落匹配，结果排在编辑器上方。章内场次是大纲，删场不删正文。启发式抽取仍可用，但 UI 不走这条路径。IPC 形状的黄金样例见 `packages/shared-types/examples.json`。匹配黄金用例见 `packages/match-fixtures/cases.json`。

## 5. 宿主 IPC（Tauri）

统一返回 `{ ok, data, error }`（`CommandResult`），camelCase。

| 命令 | 入参 | 出参 |
|---|---|---|
| `create_project` | `{ title }` | `Project` |
| `create_book` | `{ projectId, title, synopsis?, position? }` | `Book` |
| `create_chapter` | `{ projectId, bookId, title, position?, volumeId? }` | `Chapter` |
| `create_volume` | `{ projectId, bookId, title, position? }` | `Volume` |
| `create_scene` | `{ projectId, chapterId, title, position?, povEntryId? }` | `Scene` |
| `load_library` | `projectId?: string \| null` | `{ projects, activeProjectId, books, volumes, chapters, scenes }` |
| `set_active_project` | `projectId` | 同上 |
| `load_chapter` | `chapterId` | `{ chapterId, revision, text, blocks }` |
| `save_chapter` | `chapterId`, `text`, `blocks?` | 同上；有 `blocks` 时写入块序列 |
| `rename_project` / `delete_project` | `projectId`（改名另加 `title`） | `LibrarySnapshot` |
| `rename_book` / `delete_book` / `move_book` | `projectId`, `bookId`（改名加 `title`，移动加 `delta`） | `LibrarySnapshot` |
| `rename_volume` / `delete_volume` / `move_volume` | `projectId`, `volumeId`（同上） | `LibrarySnapshot` |
| `rename_scene` / `delete_scene` / `move_scene` / `set_scene_pov` | `projectId`, `sceneId`（改名加 `title`，移动加 `delta`，POV 加 `povEntryId`） | `LibrarySnapshot` |
| `rename_chapter` / `delete_chapter` / `move_chapter` | `projectId`, `chapterId`（同上） | `LibrarySnapshot` |
| `enqueue_job` | `{ projectId, operation, payload, priority }` | `{ jobId }` |
| `run_queue_step` | — | `{ executed, ... }` |
| `list_jobs` | — | `JobView[]` |
| `kernel_tools` | — | 工具自描述列表 |
| `context_hints` | `projectId`, `chapterId`, `revision`, `nearbyText`, `lookbackText?`, `generation` | `ContextHint[]`（多信号匹配预先结构） |
| `save_model_config` | provider / baseUrl / model / apiKey? | `{ saved }`；密钥进 `SecretVault`，留空保持原值 |
| `load_model_config` | — | `{ provider, baseUrl, model, apiKey: "", apiKeySet }` |
| `generate_continuation` | 章、修订、prompt、config | `ContentPatch`；config 可不带密钥 |
| `record_generation_feedback` | `projectId`, `accepted`, `aiText`, `humanText?`, `contextExcerpt?` | `PreferenceRule[]` |
| `list_preferences` | `projectId` | `PreferenceRule[]` |
| `set_preference_status` | `projectId`, `ruleId`, `disabled` | `PreferenceRule[]` |
| `list_plugins` | — | `PluginSummary[]`（打包项 `runtime` 为 `builtin`） |
| `pending_outbox_count` | — | `u32` |
| `flush_outbox_journal` | — | `{ written, path, note }`；写入应用数据目录 `sync/outbox-journal.jsonl`，不是设备间同步 |
| `propose_canon` | `chapterId` | `CanonProposal[]`（启发式抽取，非主路径） |
| `list_canon` | `projectId`, `status?` | `CanonProposal[]` |
| `review_canon_fact` | `factId`, `accept` | 更新后的 `CanonProposal` |
| `create_story_entry` | `projectId`, `kind`, `title`, `summary?` | `StoryEntry`；`title` 可写 `林晚、雾儿`，别名拆进 `aliases` |
| `list_story_entries` | `projectId` | `StoryEntry[]` |
| `delete_story_entry` | `projectId`, `id`, `kind` | — |

`training.export` 额外字段：`format`（jsonl/sharegpt/alpaca/r1）、`includeMarkup`（默认 true）、`minQuality`（默认 `usable`，丢弃 skip）。返回 `examples`、`dropped`、`qualityCounts`、`protocolVersion`（当前为 2）。每条样本的 `context` 从章首累积思考+正文，不截断。思考里的 `@` 是写作标签（`MarkupRef::Tag`），不是正史实体。写作约定见 [writing-protocol.md](writing-protocol.md)。

前端**只通过** `apps/client/src/api.ts` 的 `libraryApi` 访问作品库、结构、设置、续写、偏好、插件列表与 outbox journal。浏览器预览无 Tauri 时使用内存实现。作品库 / 队列 / 编辑会话 / 结构分别在 `hooks/useLibrary.ts`、`hooks/useQueue.ts`、`hooks/useEditorSession.ts`、`hooks/useStructure.ts`。

## 6. 改接口时的检查表

1. 改 domain 字段 → serde camelCase、SQLite 迁移、TS `types.ts`
2. 改 command 名或字段 → `libraryApi`、`command_tests`、本页表格
3. 改工具 id → 工作流模板、`OPERATION_LABELS`、本页工具表
4. `cargo test --workspace` 与 `pnpm --filter @novel-agent/client test`
