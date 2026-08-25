import { describe, expect, it } from "vitest";
import {
  ContentBlock,
  blocksToDoc,
  buildTrainingExamples,
  serializeExamples,
} from "../editor/blocks";

function block(kind: ContentBlock["kind"], text: string): ContentBlock {
  return {
    id: `00000000-0000-0000-0000-${Math.random().toString(16).slice(2, 14)}`,
    kind,
    text,
    position: 0,
    markup: [],
  };
}

describe("buildTrainingExamples", () => {
  it("pairs thinking with following body blocks", () => {
    const blocks: ContentBlock[] = [
      block("thinking", "需要铺垫主角动机"),
      block("body", "林默站在窗前。"),
      block("body", "他摩挲着旧怀表。"),
      block("thinking", "引入伏笔：怀表"),
      block("body", "表盖内侧刻着两个字。"),
    ];
    const examples = buildTrainingExamples(blocks);
    expect(examples).toHaveLength(2);
    expect(examples[0].thinking).toBe("需要铺垫主角动机");
    expect(examples[0].content).toBe("林默站在窗前。\n他摩挲着旧怀表。");
    expect(examples[1].thinking).toBe("引入伏笔：怀表");
    expect(examples[1].content).toBe("表盖内侧刻着两个字。");
  });

  it("merges consecutive thinking blocks into one thinking", () => {
    const blocks: ContentBlock[] = [
      block("thinking", "第一行思考"),
      block("thinking", "第二行思考"),
      block("body", "正文"),
    ];
    const examples = buildTrainingExamples(blocks);
    expect(examples).toHaveLength(1);
    expect(examples[0].thinking).toBe("第一行思考\n第二行思考");
  });

  it("body without preceding thinking gets empty thinking", () => {
    const blocks: ContentBlock[] = [
      block("body", "开篇正文"),
      block("thinking", "后面才思考"),
      block("body", "思考后的正文"),
    ];
    const examples = buildTrainingExamples(blocks);
    expect(examples).toHaveLength(2);
    expect(examples[0].thinking).toBe("");
    expect(examples[1].thinking).toBe("后面才思考");
  });
});

describe("serializeExamples", () => {
  const examples = [{ thinking: "想一下", content: "正文" }];

  it("serializes jsonl as thinking/content pairs", () => {
    const out = serializeExamples(examples, "jsonl");
    expect(out).toContain('"thinking":"想一下"');
    expect(out).toContain('"content":"正文"');
  });

  it("serializes sharegpt conversations", () => {
    const out = serializeExamples(examples, "sharegpt");
    expect(out).toContain('"from":"human"');
    expect(out).toContain('"from":"assistant"');
  });

  it("serializes r1 with think tags", () => {
    const out = serializeExamples(examples, "r1");
    expect(out).toBe("<think>\n想一下\n</think>\n\n正文");
  });

  it("omits think tags for empty thinking in r1", () => {
    const out = serializeExamples([{ thinking: "", content: "直接正文" }], "r1");
    expect(out).toBe("直接正文");
  });
});

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
