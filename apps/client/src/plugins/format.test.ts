import { describe, expect, it } from "vitest";
import {
  formatNameCounts,
  formatPluginResult,
  pluginIsRunnable,
  splitNames,
  uniqueNames,
} from "./format";

describe("plugin result formatting", () => {
  it("splits and dedupes names", () => {
    expect(splitNames("林默、林默，雾儿, ")).toEqual(["林默", "雾儿"]);
    expect(uniqueNames([" 林默 ", "林默", ""])).toEqual(["林默"]);
  });

  it("treats wasm hello-names as runnable", () => {
    expect(
      pluginIsRunnable({
        id: "hello-names",
        name: "人名点名",
        version: "0.1.0",
        runtime: "wasm",
        operations: ["count-names"],
      }),
    ).toBe(true);
    expect(
      pluginIsRunnable({
        id: "continuity-checker",
        name: "连续性检查",
        version: "0.1.0",
        runtime: "builtin",
        operations: ["check-chapter"],
      }),
    ).toBe(false);
  });

  it("shows appeared names first and lists zeros separately", () => {
    expect(formatNameCounts({ 林默: 3, 雾儿: 0 })).toBe("林默 × 3\n未出现：雾儿");
    expect(formatNameCounts({ 林默: 0, 雾儿: 0 })).toBe("正文里没有这些人名：林默、雾儿");
  });

  it("renders placeholder receipts in Chinese instead of JSON", () => {
    const text = formatPluginResult(
      {
        id: "continuity-checker",
        name: "连续性检查",
        version: "0.1.0",
        runtime: "builtin",
        operations: ["check-chapter"],
      },
      {
        output: { operation: "check-chapter", message: "这是内置占位回执，还没有真正执行。" },
        logs: [],
      },
    );
    expect(text).toContain("连续性检查（占位）");
    expect(text).toContain("还没有真正执行");
    expect(text).not.toContain("{");
  });
});
