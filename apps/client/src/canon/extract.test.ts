import { describe, expect, it } from "vitest";
import { extractMentions } from "./extract";

describe("extractMentions", () => {
  it("picks speakers, titles and locations", () => {
    const mentions = extractMentions(
      "林晚说道：「今夜雾很重。」两人走进雾港码头。《潮汐秘录》就放在案上。",
    );
    const names = mentions.map((item) => item.entityName);
    expect(names).toContain("林晚");
    expect(names).toContain("雾港码头");
    expect(names).toContain("潮汐秘录");
  });
});
