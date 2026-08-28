import { PluginRunResult, PluginSummary } from "../types";

/** 人名输入：顿号、逗号都算分隔。 */
export function splitNames(raw: string): string[] {
  return uniqueNames(raw.split(/[,，、]/));
}

export function uniqueNames(names: string[]): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const name of names) {
    const trimmed = name.trim();
    if (!trimmed || seen.has(trimmed)) continue;
    seen.add(trimmed);
    result.push(trimmed);
  }
  return result;
}

export function pluginIsRunnable(plugin: PluginSummary): boolean {
  return plugin.runtime === "wasm" || plugin.id === "hello-names";
}

export function formatPluginResult(plugin: PluginSummary, result: PluginRunResult): string {
  const output = (result.output ?? {}) as {
    counts?: Record<string, number>;
    message?: string;
    operation?: string;
  };
  if (plugin.id === "hello-names" && output.counts) {
    return formatNameCounts(output.counts);
  }
  if (typeof output.message === "string" && output.message.trim()) {
    const kind = pluginIsRunnable(plugin) ? "" : "（占位）";
    return `${plugin.name}${kind}\n${output.message}`;
  }
  return JSON.stringify(result.output, null, 2);
}

export function formatNameCounts(counts: Record<string, number>): string {
  const entries = Object.entries(counts);
  if (entries.length === 0) return "没有要统计的人名。";
  const appeared = entries.filter(([, count]) => count > 0);
  const missing = entries.filter(([, count]) => count === 0).map(([name]) => name);
  if (appeared.length === 0) {
    return `正文里没有这些人名：${missing.join("、")}`;
  }
  const lines = appeared.map(([name, count]) => `${name} × ${count}`);
  if (missing.length > 0) {
    lines.push(`未出现：${missing.join("、")}`);
  }
  return lines.join("\n");
}
