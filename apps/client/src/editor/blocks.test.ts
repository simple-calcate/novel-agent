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

  it("stores story tags as inline chips with tagKind, not a canon entity id", () => {
    const doc = blocksToDoc([
      {
        id: "11111111-1111-4111-8111-111111111111",
        kind: "thinking",
        text: "@人物：林默",
        position: 0,
        markup: [{ type: "tag", id: "", kind: "人物", label: "林默", note: "" }],
      },
    ]);
    const content =
      (doc.content as Array<{
        content?: Array<{ type?: string; attrs?: Record<string, string> }>;
      }>) ?? [];
    const chip = content[0]?.content?.[0];
    expect(chip?.type).toBe("markupRef");
    expect(chip?.attrs?.kind).toBe("tag");
    expect(chip?.attrs?.tagKind).toBe("人物");
    expect(chip?.attrs?.label).toBe("林默");
    expect(chip?.attrs?.id).toBe("");
  });

  it("keeps surrounding thinking text and turns @标签 into chips", () => {
    const doc = blocksToDoc([
      {
        id: "11111111-1111-4111-8111-111111111111",
        kind: "thinking",
        text: "意图：让林默出场\n@人物:林默 再写一句",
        position: 0,
        markup: [{ type: "tag", id: "", kind: "人物", label: "林默", note: "" }],
      },
    ]);
    const inline =
      (
        doc.content as Array<{
          content?: Array<{ type?: string; text?: string; attrs?: Record<string, string> }>;
        }>
      )[0]?.content ?? [];
    expect(inline).toEqual([
      { type: "text", text: "意图：让林默出场\n" },
      {
        type: "markupRef",
        attrs: { kind: "tag", id: "", tagKind: "人物", label: "林默", note: "" },
      },
      { type: "text", text: " 再写一句" },
    ]);
  });

  it("appends chips that are not already written in the thinking text", () => {
    const doc = blocksToDoc([
      {
        id: "11111111-1111-4111-8111-111111111111",
        kind: "thinking",
        text: "意图：让林默出场",
        position: 0,
        markup: [{ type: "tag", id: "", kind: "人物", label: "林默", note: "" }],
      },
    ]);
    const inline =
      (
        doc.content as Array<{
          content?: Array<{ type?: string; text?: string; attrs?: Record<string, string> }>;
        }>
      )[0]?.content ?? [];
    expect(inline.map((node) => node.type)).toEqual(["text", "markupRef"]);
    expect(inline[1]?.attrs?.label).toBe("林默");
  });
});
