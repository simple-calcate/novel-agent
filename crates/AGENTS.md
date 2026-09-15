# crates/

Rust 工作区成员。全图与切片：[docs/wiki/agents.md](../docs/wiki/agents.md)。测某个 crate：`cargo test -p <包名>`。

| 目录 | 包名 | 只做 | 别做 |
|---|---|---|---|
| `domain/` | `novel-domain` | 类型、纯计算（含 `similar` diff） | IO |
| `kernel/` | `novel-kernel` | 注册表、预算、分发、事件总线 | SQLite / HTTP |
| `extensions/` | `novel-extensions` | Tool / Provider / `Workspace` | 给 UI 绕过内核 |
| `storage/` | `novel-storage` | 迁移、单写者、仓储 | 调模型 |
| `automation/` | `novel-automation` | 信号、规则、队列状态机 | 真正执行（那是 `queue.tick`） |
| `context-hints/` | `novel-context-hints` | 段落 ↔ `StoryEntry` | 正史抽取 |
| `context-engine/` | `novel-context-engine` | `context.assemble` | 预选条 |
| `feedback-memory/` | `novel-feedback-memory` | 偏好规则纯函数 | 落库 |
| `story-model/` | `novel-story-model` | 启发式正史 | 接到预选条 |
| `plugin-host/` | `novel-plugin-host` | 清单、权限、wasmi | Android 上跑 WASM |

进子目录时再看那里的 `AGENTS.md`。
