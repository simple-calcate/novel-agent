import { joinList, t } from "../i18n";
import { PluginRunResult, PluginSummary } from "../types";

/** 人名输入：顿号、逗号都算分隔。 */
export function splitNames(raw: string): string[] {
  return uniqueNames(raw.split(/[,，、]/));
}

export function pluginDisplayName(plugin: Pick<PluginSummary, "id" | "name">): string {
  switch (plugin.id) {
    case "hello-names":
      return t("plugin.helloNames");
    case "continuity-checker":
      return t("plugin.continuity");
    case "summary-extractor":
      return t("plugin.summary");
    case "continuation-writer":
      return t("plugin.continuation");
    default:
      return plugin.name;
  }
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
    const kind = pluginIsRunnable(plugin) ? "" : t("plugin.placeholderSuffix");
    return `${pluginDisplayName(plugin)}${kind}\n${output.message}`;
  }
  return JSON.stringify(result.output, null, 2);
}

export function formatNameCounts(counts: Record<string, number>): string {
  const entries = Object.entries(counts);
  if (entries.length === 0) return t("plugin.noNames");
  const appeared = entries.filter(([, count]) => count > 0);
  const missing = entries.filter(([, count]) => count === 0).map(([name]) => name);
  if (appeared.length === 0) {
    return t("plugin.noneFound", { names: joinList(missing) });
  }
  const lines = appeared.map(([name, count]) => `${name} × ${count}`);
  if (missing.length > 0) {
    lines.push(t("plugin.missingNames", { names: joinList(missing) }));
  }
  return lines.join("\n");
}
