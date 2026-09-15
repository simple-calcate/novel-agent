# plugins/

打包清单，编译进 `novel-plugin-host`（`discover.rs` 的 `include_str!`）。

| 目录 | 软件里 |
|---|---|
| `hello-names/` | 可运行。`plugin.json` 含 `wasmBase64`。改 guest：`packages/plugin-compile/examples/hello-names.ts`，再 `pnpm --filter @novel-agent/plugin-compile compile:hello-names` |
| `continuity-checker/` `summary-extractor/` `continuation-writer/` | 占位。点运行只有中文回执 |

不要把占位项接成真连续性检查或续写。ABI 见 [docs/wiki/plugins.md](../docs/wiki/plugins.md)。
