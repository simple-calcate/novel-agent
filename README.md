# 墨枢 Novel Agent

本地优先、插件化的跨平台网文写作 Agent。

## 架构

内核 + 扩展（参考 DeepSeek harness 思想，见 [ADR 0007](docs/architecture/adr/0007-kernel-extensions.md)）。

**分层职责**：[docs/architecture/layers.md](docs/architecture/layers.md)  
**各层接口**：[docs/interfaces.md](docs/interfaces.md)

```
apps/client/          Tauri 2 + React 桌面/Android 客户端
crates/kernel/        最小内核：Provider/Tool/事件总线 + 预算硬约束的 Agent 循环
crates/extensions/    内置扩展集：模型提供方、工作流引擎、队列、上下文、插件宿主
crates/domain/        领域模型（Project、Chapter、Revision、Event、Job、Story）
crates/storage/       SQLite 持久化、迁移、单写者仓库
crates/automation/    信号检测、工作流匹配、持久化操作队列
crates/story-model/   正史模型、故事图、快照、连续性验证
crates/context-engine/ ACP 风格上下文压缩与装配
crates/context-hints/ 实时上下文浮带匹配引擎
crates/feedback-memory/ 人类纠正候选与偏好规则
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
作品库是独立的仓储/IPC 接口（`Project` → `Book` → `Chapter`），UI 经
`libraryApi` 创建与切换，不走工具表。

```rust
let kernel = Kernel::builder()
    .service(Arc::new(Mutex::new(repository))) // 注入 SQLite 仓库
    .extension(BuiltinsExtension)?             // 或逐个挑选内置扩展
    .build()?;
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
- **Revision + OperationLog + Outbox**：每章单调版本号，单写者事务，崩溃可恢复。
- **参数化信号**：`editor.idle`、`paragraph.created`、`chapter.created` 等携带完整上下文。
- **持久化队列状态机**：原子领取 + 指数退避重试 + 死信 + 崩溃后陈旧任务回收。
- **正史模型**：人物、事件、关系、知识、伏笔均带有效时间、来源和审核状态。
- **上下文浮带**：写作时持续匹配设定/钩子，三级预算，不阻塞输入。
- **分层插件**：声明式工作流 → 可信 Rust 操作 → 桌面 WASM 沙箱插件。

## 许可

GPL-3.0-or-later
