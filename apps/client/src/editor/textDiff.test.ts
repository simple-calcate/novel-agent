import { describe, expect, it } from "vitest";
import { diffTexts } from "./textDiff";

describe("textDiff", () => {
  it("reports no edits when texts match", () => {
    const diff = diffTexts("雾港来客。", "雾港来客。");
    expect(diff.summary).toBe("无改动");
    expect(diff.insertedChars).toBe(0);
    expect(diff.deletedChars).toBe(0);
  });

  it("counts inserted Chinese characters", () => {
    const diff = diffTexts("雾港来客。", "雾港来客。灯还亮着。");
    expect(diff.insertedChars).toBeGreaterThan(0);
    expect(diff.lines.some((line) => line.tag === "insert" || line.tag === "delete")).toBe(true);
    expect(diff.summary).toContain("字");
  });
});
