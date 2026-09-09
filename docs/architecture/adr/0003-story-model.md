# ADR 0003: 小说正史模型

## 状态
已接受。**产品路径已被 [ADR 0009](0009-canon-review-loop.md) 取代**：作者预先写结构，编辑器按段匹配。本 ADR 仍描述库内正史模型与抽取 API，那些表和工具还在，界面不用。

## 背景
网文的设定、剧情、人物关系远比代码上下文复杂，不能只依赖聊天记录或向量检索。

## 决策
- 正史层：CanonEntity、CanonFact、Relationship、StoryEvent、CharacterKnowledge、PlotThread。
- 每条事实带来源、置信度、审核状态、有效时间和 Revision 范围。
- 上下文装配时先硬过滤（分支、时间、POV 知识），再混合检索。
- LLM 抽取只生成候选，作者确认后进入正史。
- 启发式抽取仍可生成候选，但写作主路径是作者预先设计人物 / 设定 / 伏笔，再按当前段落匹配（见 [ADR 0009](0009-canon-review-loop.md)）。

## 后果
- 设定在库内可追溯。审核 UI、故事圣经、时间轴与伏笔看板**没有**做成产品，见 [backlog](../../wiki/backlog.md)。
- 数据库 schema 比纯文本方案复杂。
- 不要把本 ADR 的「作者确认后进入正史」写成当前界面流程。
