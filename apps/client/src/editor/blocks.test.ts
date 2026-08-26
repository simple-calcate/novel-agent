import { describe, expect, it } from "vitest";
import { blocksToDoc } from "./blocks";

describe("blocksToDoc", () => {
  it("restores thinking and body nodes with stable ids", () => {
    const doc = blocksToDoc([
      {
        id: "11111111-1111-4111-8111-111111111111",
        kind: "thinking",
        text: "先写动机",
        position: 0,
        markup: [],
      },
      {
        id: "22222222-2222-4222-8222-222222222222",
        kind: "body",
        text: "林默站在窗前。",
        position: 1,
        markup: [],
      },
    ]);
    const content = (doc.content as Array<{ type: string; attrs?: { blockId?: string } }>) ?? [];
    expect(content[0].type).toBe("thinkingBlock");
    expect(content[0].attrs?.blockId).toBe("11111111-1111-4111-8111-111111111111");
    expect(content[1].type).toBe("paragraph");
  });
});
