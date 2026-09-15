# crates/domain

包名 `novel-domain`。无 IO。切片：[docs/wiki/agents.md](../../docs/wiki/agents.md)。

| 文件 | 放什么 |
|---|---|
| `content.rs` | 作品库：`Project`…`Scene`、`LibrarySnapshot`、`ChapterBody` |
| `story.rs` | **`StoryEntry` 主路径**；`Canon*` 非主路径 |
| `diff.rs` | `similar` 正文对比 |
| `protocol.rs` | 思考 / 正文 / 拍 |
| `events.rs` / `jobs.rs` / `patches.rs` / `plugins.rs` / `ids.rs` / `work.rs` / `actor.rs` | 事件、队列、偏好、插件、Id、上下文包 |

字段 serde **camelCase**。改形状：迁移 + `apps/client/src/types.ts` + `packages/shared-types/examples.json` + [interfaces.md](../../docs/interfaces.md)。

`tests/ipc_contract.rs` 锁 interfaces §5 与 `generate_handler!`。
