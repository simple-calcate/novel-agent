# packages/

| 包 | 许可 | 用途 |
|---|---|---|
| `plugin-sdk` | MIT | `definePlugin` / 清单类型 |
| `plugin-compile` | MIT | AssemblyScript → 无导入 WASM |
| `workflow-builder` | MIT | `defineWorkflow` 与模板 |
| `event-schema` | MIT | 事件信封 |
| `match-fixtures` | 宿主 | 预选条黄金用例，Rust+TS 共用 |
| `shared-types` | 宿主 | IPC 形状 `examples.json`，两边反序列化 |

写插件看 [docs/wiki/plugins.md](../docs/wiki/plugins.md)。改匹配规则先改 `match-fixtures/cases.json`。改 IPC 字段改 `shared-types/examples.json`。
