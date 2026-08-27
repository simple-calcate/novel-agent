# 墨枢 Novel Agent

本地优先、插件化的跨平台网文写作 Agent。

**从这里读：** [docs/wiki/README.md](docs/wiki/README.md)（产品、架构、开发、术语、未做）。契约在 [docs/interfaces.md](docs/interfaces.md)，分层禁区在 [docs/architecture/layers.md](docs/architecture/layers.md)。实现以代码为准。

## 架构

内核 + 扩展（参考 DeepSeek harness 思想，见 [ADR 0007](docs/architecture/adr/0007-kernel-extensions.md)）。

**分层职责**：[docs/architecture/layers.md](docs/architecture/layers.md)  
**各层接口**：[docs/interfaces.md](docs/interfaces.md)

```
apps/client/          Tauri 2 + React 桌面/Android 客户端
crates/kernel/        最小内核：Provider/Tool/事件总线 + 预算硬约束的 Agent 循环
crates/extensions/    内置扩展集 + Workspace 应用层（作品库编排、队列入口）
crates/domain/        领域模型（Project、Chapter、Revision、Event、Job、Story）
crates/storage/       SQLite 持久化、迁移、单写者仓库
crates/automation/    信号检测、工作流匹配、持久化操作队列
crates/story-model/   正史模型、故事图、快照、连续性验证
crates/context-engine/ ACP 风格上下文压缩与装配
crates/context-hints/ 实时上下文浮带匹配引擎
crates/feedback-memory/ 人类纠正候选与偏好规则（拒绝续写后写入，下次续写带进提示）
crates/plugin-host/   插件清单、权限评估、运行时
packages/event-schema/ 版本化事件 schema
packages/plugin-sdk/   插件 SDK 与清单 JSON Schema
packages/workflow-builder/ 工作流定义与模板
plugins/              内置插件清单
docs/architecture/adr/ 架构决策记录
```

内核只做四件事：组装模型请求、按预算消费输出流、分发工具调用、派发领域
事件。其余一切（DeepSeek/OpenAI 兼容提供方、工作流引擎、队列执行、上下文
浮带、插件宿主）都是注册进内核的扩展，同名注册即可覆盖内置实现。
作品库是独立的仓储/IPC 接口（`Project` → `Book` → 可选 `Volume` → `Chapter` → 可选 `Scene`），UI 经
`libraryApi` 创建与切换，不走工具表。

```rust
let kernel = Kernel::builder()
    .service(Arc::new(StorageHandle::open(database)?))
    .extension(BuiltinsExtension)?             // 或逐个挑选内置扩展
    .build()?;
let workspace = Workspace::new(&kernel);
```

## 开发

```bash
# 安装依赖
npm install -g pnpm
pnpm install

# Rust 编译检查
cargo check --workspace

# 前端开发
pnpm --filter @novel-agent/client dev

# Tauri 桌面
cd apps/client && pnpm tauri dev
```

## 核心设计

- **内核 + 扩展**：内核极小且稳定（无 HTTP / SQLite 依赖），预算（时间/token）在内核硬约束；业务全部在扩展层，可覆盖、可增删。
- **Revision + OperationLog + Outbox**：每章单调版本号，单写者事务，写路径同时入 outbox；本机可写成 JSONL，同步传输仍是后续阶段。
- **参数化信号**：`editor.idle`、`paragraph.created`、`chapter.created` 等携带完整上下文。
- **持久化队列状态机**：原子领取 + 指数退避重试 + 死信 + 崩溃后陈旧任务回收。
- **正史模型**：人物、事件、关系、知识、伏笔均带有效时间、来源和审核状态。
- **预先结构**：人物、设定、伏笔由作者写入，编辑器按当前段落匹配并显示在上方预选条。
- **写作协议**：思考是写给自己看的便签，正文是写给读者看的小说。空行 `Tab` 切到思考，思考里 `@` 点人物/伏笔。约定见 [docs/writing-protocol.md](docs/writing-protocol.md)，完整样章见 [docs/examples/fog-harbor.md](docs/examples/fog-harbor.md)；软件里点「打开示例章节」可装进作品库。
- **章内场次**：当前章可列大纲场，删场不删正文。
- **上下文浮带**：写作时按当前段落匹配预先结构，本地规则没命中再做词汇检索；可钉住或忽略。
- **密钥分离**：模型 API Key 走系统密钥链（或本机 0600 文件），不进 SQLite。
- **分层插件**：声明式工作流 → 可信 Rust 操作 → 桌面 WASM 沙箱插件。

## 许可

宿主（应用、内核、仓储、编辑器、匹配）为**专有许可**，见 [LICENSE](LICENSE)。
写插件用的接口是 **MIT**：`packages/plugin-sdk`、`packages/event-schema`、`packages/workflow-builder`。说明见 [许可](docs/wiki/licensing.md) 与 [ADR 0012](docs/architecture/adr/0012-host-proprietary-plugin-mit.md)。作者保证见 [trust.md](docs/wiki/trust.md)。
