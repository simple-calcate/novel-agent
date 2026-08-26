# ADR 0007: 内核 + 扩展架构（参考 DeepSeek harness 思想）

## 状态
已接受

## 背景
原 `agent-runtime` 把 Provider 抽象、续写循环和具体 HTTP 客户端耦在一起，
`automation::runner` 的任务执行器是硬编码 HashMap，队列任务执行后状态永不
更新（会无限重复执行），`crates/agent-runtime` 无法被第三方替换或增强。
宿主（Tauri）直接依赖每一个功能 crate，新增能力必须改宿主代码。

DeepSeek 公开分享过的 agent harness 设计思想给了可借鉴的骨架：

1. **最小内核**：harness 只负责「组装请求 → 流式消费 → 分发工具 → 派发事件」
   这个循环，不含任何业务逻辑。
2. **一切可插拔**：模型提供方、工具、事件钩子都是注册进内核的扩展。
3. **预算是一等公民**：轮数 / token / 时间在内核强制执行，不依赖 Provider 自觉。
4. **流式优先**：所有模型 IO 走流。
5. **事件驱动可观测**：生命周期事件可订阅。

## 决策

新增两个 crate，删除 `agent-runtime`：

- `crates/kernel`（仅依赖 `novel-domain`）：
  - `ProviderRegistry` + `ProviderFactory`：按名字创建 `ModelProvider`；
  - `ToolRegistry` + `Tool` trait：所有可执行能力（含队列操作）统一为工具，
    同名注册可覆盖内置实现；
  - `EventBus` + `EventSubscriber`：领域事件按类型路由，单订阅者失败不阻断其他；
  - `Services` 类型注册表：依赖倒置，内核不依赖 storage，宿主注入
    `Arc<StorageHandle>`，扩展按类型取回；
  - `BudgetGuard` + `run_continuation`：时间与输出 token 预算硬约束，
    超限截断并标记 `truncated`，可选发布 `agent.finished` 事件。
- `crates/extensions`（内置扩展集，实现 `Extension` trait）：
  - providers：echo + OpenAI 兼容（deepseek/openai/moonshot/ollama/custom），
    含跨分块缓冲的增量 SSE 解析器；
  - workflow：事件订阅 → 记录 → 规则匹配（含冷却）→ 稳定幂等键入队；
  - queue：`queue.tick` 工具驱动任务状态机（claim → 执行 → succeeded /
    指数退避 / 死信），`QueuePolicy` 服务可定制退避与陈旧回收阈值；
  - core-tools / hints / context-assembly / plugin-host：原有能力全部工具化。

宿主组装方式：

```rust
let kernel = Kernel::builder()
    .service(Arc::new(Mutex::new(repository)))
    .extension(BuiltinsExtension)?   // 或逐个挑选/覆盖
    .build()?;
```

Tauri 命令层只做参数翻译，作品库走 `Workspace`，Agent/队列走 `kernel.call_tool` /
`kernel.dispatch` / `kernel.run_continuation`。见 [ADR 0008](0008-workspace-storage-handle.md)。

## 修复的缺陷

- 队列任务执行后状态永不更新（无限重跑）→ claim/complete/fail 状态机 +
  指数退避 + 死信 + 崩溃后陈旧 running 任务回收。
- SSE 解析丢数据：一个网络分块只处理第一条 data 行、跨分块断行不缓冲、
  不支持 CRLF → 增量 SSE 解析器。
- `apply_operation` 字节偏移落在 UTF-8 字符中间会 panic → 对齐字符边界。
- 幂等键用计数器而非动作名 → `stable_idempotency_key(event, workflow, action)`。
- `cooldown_ms` 从未生效 → `workflow_fired` 表 + 冷却检查。
- `create_chapter` 在 book 不存在时静默成功 → 检查受影响行数。
- `commit_patch` 向 operation_log 写空 project_id → join 查询真实项目。
- `depends_on` 被忽略 → claim 时校验依赖全部 succeeded。
- 续写预算（时间/token）完全不生效 → BudgetGuard 硬约束。
- `context_hints` 对非法 projectId 静默替换为随机 ID → 返回错误。
- ACP 压缩块 `direct_message_ids` 为空时索引越界 → 跳过空块。
- 每次请求新建 `reqwest::Client` → `OnceLock` 共享。
- （补测试时发现）开启 `include_usage` 后 usage 统计帧在 `finish_reason` 之后
  到达，旧翻译把 finish 帧标记为终止信号导致 token 统计永远丢失 →
  终止只由 usage 帧 / `[DONE]` / 流结束决定。
- （补测试时发现）`PluginManifest.id` 建模为 UUID，但插件生态（内置清单、
  TS SDK、manifest schema）全部使用字符串 slug，导致内置插件清单无法安装 →
  改为 `String` slug。

## 后果

- 内核稳定且极小（无 HTTP、无 SQLite 依赖），业务变化集中在扩展层。
- 第三方可以覆盖任何内置工具/提供方，或注册新的事件订阅者。
- 队列操作名与工具名统一（`document.save` 等），工作流动作自动路由到工具。
- 模型 API Key 仍以明文存于本地 SQLite（app_settings），后续应接入系统密钥链。
- WASM 插件沙箱（ADR 0004 第三层）今后可作为又一个扩展接入 `plugin.operation`。
