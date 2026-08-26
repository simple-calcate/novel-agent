import { describe, expect, it } from "vitest";
import {
  authorExportSummary,
  countMissingThinking,
  guideCopy,
  writerModeFromParent,
} from "./guide";

describe("writerModeFromParent", () => {
  it("maps thinking block and paragraph", () => {
    expect(writerModeFromParent("thinkingBlock")).toBe("thinking");
    expect(writerModeFromParent("paragraph")).toBe("body");
  });
});

describe("guideCopy", () => {
  it("tells the author to finish thinking then write body", () => {
    const copy = guideCopy({ mode: "thinking", missingThinkingBeats: 0 });
    expect(copy.title).toContain("思考");
    expect(copy.body).toContain("Tab");
  });

  it("warns when body beats have no thinking", () => {
    const copy = guideCopy({ mode: "body", missingThinkingBeats: 2 });
    expect(copy.body).toContain("2 段");
    expect(copy.body).toContain("没有思考");
  });
});

describe("authorExportSummary", () => {
  it("explains skipped body-only beats in plain language", () => {
    expect(authorExportSummary(3, 2)).toBe(
      "已导出 3 段。另有 2 段只有正文、没有思考，没有导出。",
    );
    expect(authorExportSummary(0, 2)).toContain("没有导出");
    expect(authorExportSummary(4, 0)).toContain("4 段完整");
  });

  it("counts emptyThinking skips", () => {
    expect(
      countMissingThinking([
        { skipReasons: ["emptyThinking"] },
        { skipReasons: ["bodyTooShort"] },
        { skipReasons: ["emptyThinking", "bodyTooShort"] },
      ]),
    ).toBe(2);
  });
});
