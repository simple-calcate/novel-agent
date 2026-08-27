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

完整例子：`packages/plugin-sdk/src/examples/hello-names.ts`（清单与 TypeScript 参考实现）。

## 编成 WASM

桌面若清单带 `wasmBase64`，`plugin.operation` 在 wasmi 里跑。Guest 必须：

- 导出 `memory`
- 导出 `plugin_execute(i32,i32)->(i32,i32)`，或 `plugin_execute(i32,i32)->i64`（高 32 位指针、低 32 位长度）
- 读宿主写入的 JSON：`{"operation":"...","input":{...}}`（写在已有线性内存之后，不会盖掉静态数据）
- 返回 JSON：`{"output":{...},"logs":["..."]}` 的指针和长度
- 无 WASI、无宿主导入；有燃料上限。Android 忽略 WASM。

TypeScript 不能直接进沙箱。脚手架是 MIT 包 `@novel-agent/plugin-compile`：用 AssemblyScript（TS 子集）写 guest，编成无导入的 wasm32。

```bash
pnpm --filter @novel-agent/plugin-compile compile:hello-names
```

或：

```ts
import { compileGuest, packPlugin } from "@novel-agent/plugin-compile";
import { helloNames } from "@novel-agent/plugin-sdk";

const wasm = await compileGuest("examples/hello-names.ts");
console.log(packPlugin(helloNames, wasm));
```

Guest 入口示例：`packages/plugin-compile/examples/hello-names.ts`。可复用 `assembly/execute.ts` 与 `assembly/json.ts`。语法是 AssemblyScript，不是完整 TypeScript（没有 DOM、没有 npm 包）。wasmi 回归固件在 `crates/plugin-host/tests/fixtures/hello-names.wasm`，改 guest 后重新 `compile:hello-names` 再拷过去。

签名与商店见阶段 3。多数人仍应只用 `defineWorkflow`，不必写 guest。
