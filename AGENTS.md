# 给改本仓库的智能体

先读 [docs/wiki/agents.md](docs/wiki/agents.md)，**按五个阶段走**，不要把 wiki 一次读完。每阶段回看那一页的仓库树，再只打开本阶段列出的文件。

- 实现以代码为准。签名：[docs/interfaces.md](docs/interfaces.md)。禁区：[docs/architecture/layers.md](docs/architecture/layers.md)。
- **结构**（`story_entries`）是写作主路径；**正史**（canon 抽取）库内仍在，界面不用。不要把抽取接到预选条。
- 用户点一下就能做：`Workspace` → Tauri command → `apps/client/src/api.ts` 的 `libraryApi`。不要先做成 Tool。宿主 command 只译 JSON，不写业务。
- 前端作品库路径只走 `libraryApi`。浏览器预览是内存；桌面才是 SQLite + 密钥库。匹配、修订 diff、插件运行常常是两套实现。
- 改 command 必须同步 interfaces **§5** 与 `apps/client/src-tauri/src/lib.rs` 的 `generate_handler!`（`ipc_contract` 会锁）。
- 产品阶段 1：本机 SQLite。同步传输、冲突 UI、Android APK、LLM 重排、正史抽取 UI、插件商店不是当前默认任务。见 [docs/wiki/backlog.md](docs/wiki/backlog.md)。
