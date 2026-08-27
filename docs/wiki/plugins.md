# 写插件

接口是 MIT：`packages/plugin-sdk`、`packages/event-schema`、`packages/workflow-builder`。宿主（编辑器、匹配、续写、SQLite）是专有软件，见 [LICENSE](../../LICENSE) 与 [ADR 0012](../architecture/adr/0012-host-proprietary-plugin-mit.md)。

## 多数人：声明式工作流

不要写 WASM。用触发器 + 已有动作：

```ts
import { defineWorkflow } from "@novel-agent/workflow-builder";

export const idle = defineWorkflow({
  id: "idle-save",
  name: "停笔后保存并刷新索引",
  enabled: true,
  trigger: "editor.idle",
  conditions: [{ path: "idleMs", operator: "gte", value: 1800 }],
  actions: [{ type: "saveDocument" }, { type: "rebuildIndex" }],
  priority: 100,
  cooldownMs: 5000,
});
```

模板还在 `createIdleWorkflow` / `createChapterWorkflow`。应用里工作流页目前是内置模板，可视化编辑器还没有。

## 要自定义操作：清单 + 沙箱

```ts
import { definePlugin, toManifestJson } from "@novel-agent/plugin-sdk";

const plugin = definePlugin({
  id: "hello-names",
  name: "人名点名",
  version: "0.1.0",
  apiVersion: 1,
  platforms: ["linux", "windows"],
  operations: [
    {
      name: "count-names",
      inputSchema: { type: "object" },
      outputSchema: { type: "object" },
      triggers: ["editor.idle"],
    },
  ],
  requestedCapabilities: [{ kind: "readSelection" }, { kind: "log" }],
});

console.log(toManifestJson(plugin));
```

完整例子：`packages/plugin-sdk/src/examples/hello-names.ts`。

桌面若清单带 `wasmBase64`，`plugin.operation` 在 wasmi 里跑：导出 `memory` 与 `plugin_execute(i32,i32)->(i32,i32)`，JSON 进 JSON 出。无 WASI、无宿主导入、有燃料上限。Android 忽略 WASM。

TS 编成 WASM 的脚手架还没有；现在先用 `definePlugin` 把清单写对。签名与商店见阶段 3。
