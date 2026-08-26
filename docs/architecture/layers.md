# 分层与职责

墨枢按「内核稳定、业务可替换」切开。每一层只依赖它下面的接口，不跨层抓实现。

```
┌─────────────────────────────────────────────┐
│  UI（React）                                 │
│  apps/client/src  · libraryApi / hooks       │
├─────────────────────────────────────────────┤
│  宿主 IPC（Tauri commands）                  │
│  apps/client/src-tauri  · 参数翻译，不写业务  │
├─────────────────────────────────────────────┤
│  应用层 Workspace                            │
│  novel-extensions::Workspace                 │
│  作品库 / 设置 / 入队 / 续写配置              │
├─────────────────────────────────────────────┤
│  内核 Kernel                                 │
│  call_tool / dispatch / run_continuation     │
├──────────────┬──────────────────────────────┤
│  扩展        │  领域类型（无 IO）             │
│  providers   │  Project → Book → Chapter     │
│  queue/tools │  Event / Job / Patch          │
├──────────────┴──────────────────────────────┤
│  单写者 StorageHandle → Repository（SQLite） │
└─────────────────────────────────────────────┘
```

| 层 | Crate / 包 | 允许做的事 | 禁止做的事 |
|---|---|---|---|
| Domain | `novel-domain` | 定义实体、事件、错误 | 打开数据库、发 HTTP |
| Kernel | `novel-kernel` | 注册表、预算、工具分发、事件总线 | 依赖 SQLite / reqwest |
| Extensions | `novel-extensions` | 实现 Tool / Provider / Subscriber；`Workspace` 编排 | 绕过内核直接给 UI 用 |
| Storage | `novel-storage` | 迁移、CRUD、修订提交、`StorageHandle` | 调用模型、解析 UI 事件 |
| Automation | `novel-automation` | 信号、规则匹配、队列状态机 | 执行具体操作（由 `queue.tick` 做） |
| Host | `src-tauri` | 把 JSON 译成领域类型，调 `Workspace` 或内核 | 复制扩展里的业务分支 |
| UI | `apps/client/src` | 渲染、通过 `libraryApi` 说话 | 直接拼 SQL 或工具名散落各处 |

新增能力时的落点：

1. **数据形状变了** → `novel-domain` + 迁移 + `docs/interfaces.md`
2. **可被 Agent/队列调用** → 实现 `Tool`，在扩展里 `register_tool`
3. **用户点按钮就能做** → `Workspace` 方法 + Tauri command + `libraryApi` 方法 + 界面
4. **只改装配** → `Kernel::builder().extension(...)` 或覆盖同名工具

接口清单见 [interfaces.md](../interfaces.md)。决策记录见 [adr/](adr/)，含 [ADR 0008](adr/0008-workspace-storage-handle.md) 与 [ADR 0009](adr/0009-writing-protocol.md)。
