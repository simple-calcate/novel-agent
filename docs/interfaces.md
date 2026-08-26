# 接口清单

分层职责见 [architecture/layers.md](architecture/layers.md)。本文件列出**当前稳定契约**：改这些签名时请同步测试与本页。

## 1. 领域：作品层级

```
Project（作品） 1—n Book（书/卷） 1—n Chapter（章）
```

| 类型 | 含义 | 关键字段 |
|---|---|---|
| `Project` | 一部作品的工作区 | `id`, `title`, `createdAt`, `updatedAt` |
| `Book` | 书或分卷 | `id`, `projectId`, `title`, `synopsis`, `position` |
| `Chapter` | 可修订的正文单位 | `id`, `bookId`, `title`, `position`, `currentRevision`, `status` |

创建书时 `position = 0` 表示自动排到该作品末尾；章节同理。书必须属于已存在的作品，章必须属于该作品下的书，否则仓储返回 `NotFound`。

领域事件（`EventKind::as_str`）：

| 事件 | 何时发出 |
|---|---|
| `project.created` / `project.renamed` / `project.deleted` | 作品增删改 |
| `book.created` / `book.renamed` / `book.deleted` / `book.reordered` | 书 |
| `chapter.created` / `chapter.renamed` / `chapter.deleted` / `chapter.reordered` | 章 |
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
| `service::<T>()` | 取注入服务（如 `Mutex<Repository>`） |
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

## 4. 仓储：`Repository`

作品库：

- `create_project(title)`
- `create_book(project_id, title, synopsis, position)`
- `create_chapter(project_id, book_id, title, position)`
- `list_projects` / `list_books` / `list_chapters`
- `rename_project` / `delete_project`
- `rename_book` / `delete_book` / `move_book(delta)`
- `rename_chapter` / `delete_chapter` / `move_chapter(delta)`
- `save_chapter_snapshot` / `save_block_sequence` / `block_sequence`
- `chapter_text` / `current_revision` / `commit_patch`

设置：`save_setting` / `get_setting`；当前作品键 `SETTING_ACTIVE_PROJECT`。

单写者：调用方持 `Mutex<Repository>`。**先释放锁再 `kernel.dispatch`**，避免订阅者再次加锁死锁。

## 5. 宿主 IPC（Tauri）

统一返回 `{ ok, data, error }`（`CommandResult`），camelCase。

| 命令 | 入参 | 出参 |
|---|---|---|
| `create_project` | `{ title }` | `Project` |
| `create_book` | `{ projectId, title, synopsis?, position? }` | `Book` |
| `create_chapter` | `{ projectId, bookId, title, position? }` | `Chapter` |
| `load_library` | `projectId?: string \| null` | `{ projects, activeProjectId, books, chapters }` |
| `set_active_project` | `projectId` | 同上 |
| `load_chapter` | `chapterId` | `{ chapterId, revision, text, blocks }` |
| `save_chapter` | `chapterId`, `text`, `blocks?` | 同上；有 `blocks` 时写入块序列 |
| `rename_project` / `delete_project` | `projectId`（改名另加 `title`） | `LibrarySnapshot` |
| `rename_book` / `delete_book` / `move_book` | `projectId`, `bookId`（改名加 `title`，移动加 `delta`） | `LibrarySnapshot` |
| `rename_chapter` / `delete_chapter` / `move_chapter` | `projectId`, `chapterId`（同上） | `LibrarySnapshot` |
| `enqueue_job` | `{ projectId, operation, payload, priority }` | `{ jobId }` |
| `run_queue_step` | — | `{ executed, ... }` |
| `list_jobs` | — | `JobView[]` |
| `kernel_tools` | — | 工具自描述列表 |
| `context_hints` | 见 `HintRequest` | `ContextHint[]` |
| `generate_continuation` | 章、修订、prompt、config | `ContentPatch` |

`training.export` 额外字段：`format`（jsonl/sharegpt/alpaca/r1）、`includeMarkup`（默认 true）、`minQuality`（默认 `usable`，丢弃 skip）。返回 `examples`、`dropped`、`qualityCounts`、`protocolVersion`。写作约定见 [writing-protocol.md](writing-protocol.md)。

前端**只通过** `apps/client/src/api.ts` 的 `libraryApi` 访问作品库。浏览器预览无 Tauri 时使用内存实现，契约与上表相同。

## 6. 改接口时的检查表

1. 改 domain 字段 → serde camelCase、SQLite 迁移、TS `types.ts`
2. 改 command 名或字段 → `libraryApi`、`command_tests`、本页表格
3. 改工具 id → 工作流模板、`OPERATION_LABELS`、本页工具表
4. `cargo test --workspace` 与 `pnpm --filter @novel-agent/client test`
