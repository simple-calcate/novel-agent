# apps/client

`@novel-agent/client`。UI + 宿主。切片：[docs/wiki/agents.md](../../docs/wiki/agents.md)。

- 作品库 / 结构 / 设置 / 续写 / 插件 / 修订：**只**通过 `src/api.ts` 的 `libraryApi`。每个方法都有 Tauri / 内存两支，两支都要写。
- 浏览器预览：`pnpm --filter @novel-agent/client dev`，数据在内存，关页即丢。
- 桌面：`cd apps/client && pnpm tauri dev`。
- 队列例外：`src/hooks/useQueue.ts` 直接 `invoke`，听 `queue:changed`。
- 预选条钉住/忽略：`localStorage` `moshu.hintPrefs.${projectId}`。
- 匹配：`src/structure/match.ts` + `packages/match-fixtures/cases.json`；桌面另有 Rust 引擎。
- 示例章：改 `src/editor/examples/fog-harbor.json`，不要写死人名。
- 测试：`pnpm --filter @novel-agent/client test` 与 `typecheck`。

宿主在 `src-tauri/`，看那里的 `AGENTS.md`。
