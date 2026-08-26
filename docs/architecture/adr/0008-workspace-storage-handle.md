# ADR 0008: 应用层 Workspace + 单写者 StorageHandle

## 状态
已接受

## 背景
ADR 0002 要求所有写操作经过单写者；ADR 0007 要求宿主只做 JSON 翻译。
实现里这两条都靠约定：宿主和扩展直接 `Mutex<Repository>::lock()`，文档写着
「先释放锁再 `kernel.dispatch`」。漏一次就会在同步订阅者里再次加锁死锁。
作品库 CRUD、设置、手动入队也散落在 `src-tauri/lib.rs` 里，和 IPC 翻译缠在一起。

## 决策

- `novel-storage::StorageHandle` 是注入内核服务表的唯一仓库入口。
  同线程嵌套 `with` / `execute` 返回 `StorageError::Reentrancy`，而不是卡住。
- `novel-extensions::Workspace` 是宿主调用的应用层：作品库编排、设置、
  队列入口、续写配置解析。**写库闭包返回之后**再 `kernel.dispatch`。
- Tauri 命令只解析 camelCase JSON、调用 `Workspace` 或 `kernel.call_tool`、
  把结果封进 `{ ok, data, error }`。
- 前端 `App.tsx` 按作品库 / 队列 / 编辑会话拆 hook，不再把全部状态堆在一个组件。

## 后果

- 订阅者在 `dispatch` 期间可以再次进入 writer（例如工作流记录事件并入队）。
- 若有人在 `StorageHandle::with` 里面 `dispatch`，会立刻得到 Reentrancy 错误，
  而不是生产环境死锁。
- 作品库仍不走工具表（与 README 一致）；Agent / 队列操作仍走 `call_tool`。
