# 墨枢手册

这是当前代码的入口。ADR 记录「为什么这样切」，[interfaces.md](../interfaces.md) 记录稳定契约；本手册只写**现在怎么用、怎么改、还缺什么**。实现以代码为准。

文档放在仓库 `docs/wiki/` 里，跟代码一起审、一起发，而不是 GitHub Wiki。改产品行为时先改代码，再改本手册对应页，避免 ADR 与界面各说各话。

| 页 | 读它为了 |
|---|---|
| [产品](product.md) | 作者侧：界面、作品库、预先结构、段落预选、续写与偏好、密钥 |
| [作者保证](trust.md) | 稿子在哪、停订能否打开、插件权限 |
| [写插件](plugins.md) | `definePlugin` / `defineWorkflow`；MIT 接口 |
| [许可](licensing.md) | 宿主专有，SDK MIT |
| [架构](architecture.md) | 分层、写路径、匹配、outbox、密钥库、调用链 |
| [开发](development.md) | 怎么跑、测什么、改一处能力动哪些文件 |
| [术语](glossary.md) | 作品 / 正史 / 预选条 / Workspace 等容易混的词 |
| [未做](backlog.md) | 表有了但产品没有、以及不要当小重构的大件 |

## 先读哪一页

- 想写小说、对代码没兴趣 → [产品](product.md) 和 [作者保证](trust.md)
- 想写插件 → [写插件](plugins.md)
- 要改功能、怕切错层 → [架构](architecture.md) + [layers.md](../architecture/layers.md)
- clone 下来要跑起来 → [开发](development.md)
- 不确定「正史」是不是主路径 → [术语](glossary.md)，再看 [ADR 0009](../architecture/adr/0009-canon-review-loop.md)

## 决策记录

| ADR | 一句话 |
|---|---|
| [0001](../architecture/adr/0001-local-first.md) | 本地优先，云不是可用性前提 |
| [0002](../architecture/adr/0002-revision-and-outbox.md) | Revision + 同事务 outbox |
| [0003](../architecture/adr/0003-story-model.md) | 正史模型仍在库内，**不是**写作主路径 |
| [0004](../architecture/adr/0004-plugin-sandbox.md) | 插件三层；桌面 wasmi，Android 内置 |
| [0005](../architecture/adr/0005-context-hints.md) | 编辑器上方浮带；本地匹配 + 词汇检索 |
| [0006](../architecture/adr/0006-android-strategy.md) | Android 伴侣；CI 检查无 C 依赖 crate |
| [0007](../architecture/adr/0007-kernel-extensions.md) | 内核极小，业务在扩展 |
| [0008](../architecture/adr/0008-workspace-storage-handle.md) | Workspace + 单写者 StorageHandle |
| [0009](../architecture/adr/0009-canon-review-loop.md) | 作者预先写结构，按当前段落匹配 |
| [0010](../architecture/adr/0010-secret-vault.md) | API Key 不进 SQLite |
| [0011](../architecture/adr/0011-writing-protocol.md) | 思考 / 正文分层；拍为导出原子 |
| [0012](../architecture/adr/0012-host-proprietary-plugin-mit.md) | 宿主专有，插件接口 MIT |

同步阶段见 [sync-and-cloud.md](../sync-and-cloud.md)（阶段 1：本机 SQLite + 可选 JSONL journal）。分层禁区见 [layers.md](../architecture/layers.md)。

旧文若仍写「从正文抽人物再审核」，以 0009 和 [产品](product.md) 为准。
