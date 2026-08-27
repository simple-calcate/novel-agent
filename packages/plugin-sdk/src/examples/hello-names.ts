import { definePlugin, PluginContext, PluginResult } from "../index";

/**
 * 第三方插件示例：只读当前选区，统计给定人名出现次数。
 * `definePlugin` 产出清单；进桌面沙箱还要把逻辑编成 WASM。
 * Android 忽略 WASM，只跑声明式工作流与内置操作。
 */
export const helloNames = definePlugin({
  id: "hello-names",
  name: "人名点名",
  version: "0.1.0",
  apiVersion: 1,
  platforms: ["linux", "windows"],
  operations: [
    {
      name: "count-names",
      description: "统计选区里出现了多少次给定人名",
      inputSchema: {
        type: "object",
        required: ["names"],
        properties: {
          names: { type: "array", items: { type: "string" } },
          selection: { type: "string" },
        },
      },
      outputSchema: {
        type: "object",
        required: ["counts"],
        properties: {
          counts: { type: "object" },
        },
      },
      triggers: ["editor.idle"],
    },
  ],
  requestedCapabilities: [{ kind: "readSelection" }, { kind: "log" }],
});

export function countNames(
  selection: string,
  names: string[],
  ctx?: Pick<PluginContext, "log">,
): PluginResult {
  const counts: Record<string, number> = {};
  for (const name of names) {
    if (!name) continue;
    let found = 0;
    let from = 0;
    while (name.length > 0) {
      const index = selection.indexOf(name, from);
      if (index < 0) break;
      found += 1;
      from = index + name.length;
    }
    counts[name] = found;
  }
  ctx?.log("info", `counted ${Object.keys(counts).length} names`);
  return { output: { counts }, logs: ["hello-names"] };
}
