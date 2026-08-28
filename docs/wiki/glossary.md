# 术语

| 词 | 在本仓库里的意思 |
|---|---|
| **作品 / Project** | 一部长篇的工作区。下面有书。 |
| **书 / Book** | 作品下的一本书。 |
| **卷 / Volume** | 书下的可选分组。删卷不删章。 |
| **章 / Chapter** | 可修订的正文单位，带单调 `currentRevision`。 |
| **场 / Scene** | 章内大纲标题。删场不删正文。可选 POV 指向人物结构条目。 |
| **结构 / StoryEntry** | 作者预先写的人物、设定、伏笔。存在 `story_entries`。这是写作主路径。 |
| **预选条 / ContextRail** | 编辑器上方按当前段落排出的结构卡片。无命中则隐藏。可钉住或忽略。 |
| **浮带 / context hints** | 与预选条同一条产品线；工具 id 是 `context.hints`。 |
| **正史 / Canon** | 启发式抽取的实体与事实候选，走 `canon_*` 表和审核 API。库内仍在，**界面不用**。 |
| **Workspace** | `novel-extensions` 里的应用层，宿主只应通过它做作品库 / 设置 / 续写编排。 |
| **StorageHandle** | 单写者仓储入口。持锁时禁止 `dispatch`。 |
| **Kernel** | 只做组装请求、预算截断、工具分发、事件派发。 |
| **Outbox** | 与业务写同一事务入队的变更记录。可写成 JSONL；没有设备间传输。 |
| **插件 SDK** | MIT 的清单与工作流类型。宿主专有。 |
| **SecretVault** | API Key 存放处（密钥链或 0600 文件），不进 SQLite。 |
| **偏好 / PreferenceRule** | 拒绝续写后记下的规则，下次续写进 system prompt。可在 Agent 页停用。 |
| **浏览器预览** | Vite 前端，内存库，无 Tauri。 |
| **叠放 PR** | 功能分支互相为 base，最终才进 `main`。面向非 `main` 的 PR 不跑现有 CI workflow。 |
