# 墨枢 Novel Agent

本地优先、插件化的跨平台网文写作 Agent。

## 架构

```
apps/client/          Tauri 2 + React 桌面/Android 客户端
crates/domain/        领域模型（Project、Chapter、Revision、Event、Job、Story）
crates/storage/       SQLite 持久化、迁移、单写者仓库
crates/automation/    信号检测、工作流匹配、持久化操作队列
crates/story-model/   正史模型、故事图、快照、连续性验证
crates/context-engine/ ACP 风格上下文压缩与装配
crates/context-hints/ 实时上下文浮带匹配引擎
crates/feedback-memory/ 人类纠正候选与偏好规则
crates/agent-runtime/ 模型 Provider 抽象与 Agent 运行时
crates/plugin-host/   插件清单、权限评估、运行时
packages/event-schema/ 版本化事件 schema
packages/plugin-sdk/   插件 SDK 与清单 JSON Schema
packages/workflow-builder/ 工作流定义与模板
plugins/              内置插件清单
docs/architecture/adr/ 架构决策记录
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

- **Revision + OperationLog + Outbox**：每章单调版本号，单写者事务，崩溃可恢复。
- **参数化信号**：`editor.idle`、`paragraph.created`、`chapter.created` 等携带完整上下文。
- **正史模型**：人物、事件、关系、知识、伏笔均带有效时间、来源和审核状态。
- **上下文浮带**：写作时持续匹配设定/钩子，三级预算，不阻塞输入。
- **分层插件**：声明式工作流 → 可信 Rust 操作 → 桌面 WASM 沙箱插件。

## 许可

GPL-3.0-or-later
