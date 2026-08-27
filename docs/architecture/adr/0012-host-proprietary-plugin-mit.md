# ADR 0012: 宿主专有，插件接口 MIT

## 状态
已接受

## 背景
墨枢是个人作品。需要让作者放心把稿子放进软件，让开发者能写插件，同时不把整仓按 GPL 放开以免被原样做成竞品。GPL 强制衍生作品开源，挡不住叉一份改名，也挡不住闭源白嫖以外的复制。

## 决策
- 应用程序、内核、扩展、仓储、编辑器与匹配实现使用根目录 [LICENSE](../../LICENSE)（专有）。
- `packages/plugin-sdk`、`packages/event-schema`、`packages/workflow-builder` 使用 MIT，作为写插件与声明式工作流的公开接口。
- 作者保证写进产品：[trust.md](../../wiki/trust.md)。默认同步关闭；付费能力到期不得锁本机正文。
- 第三方插件用 `definePlugin` 产出清单；桌面 WASM 仍无 WASI。声明式工作流用 `defineWorkflow`，不写 WASM。

## 后果
- 公开仓库里能看到宿主源码，不等于授予开源许可。若要减少被抄实现，需自行把宿主仓库改为私有；MIT 接口仍应可单独发布。
- 插件签名、商店抽成仍是 [sync-and-cloud.md](../../sync-and-cloud.md) 阶段 3，本 ADR 不假装已有市场。
