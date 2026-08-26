# ADR 0003: 小说正史模型

## 状态
已接受

## 背景
网文的设定、剧情、人物关系远比代码上下文复杂，不能只依赖聊天记录或向量检索。

## 决策
- 正史层：CanonEntity、CanonFact、Relationship、StoryEvent、CharacterKnowledge、PlotThread。
- 每条事实带来源、置信度、审核状态、有效时间和 Revision 范围。
- 上下文装配时先硬过滤（分支、时间、POV 知识），再混合检索。
- LLM 抽取只生成候选，作者确认后进入正史。
- 首版用本地启发式代替 LLM（见 [ADR 0009](0009-canon-review-loop.md)）；写入路径仍是候选 → 确认。

## 后果
- 设定可追溯、可审核、可回滚。
- 数据库 schema 比纯文本方案复杂。
- 需要投入 UI 展示故事圣经、时间轴与伏笔看板。
