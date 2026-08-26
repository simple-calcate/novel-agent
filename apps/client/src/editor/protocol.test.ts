import { describe, expect, it } from "vitest";
import {
  buildTrainingExamples,
  filterExamples,
  parseThinkingSlots,
  serializeExamples,
  STORY_TAG_KINDS,
  WRITING_PROTOCOL_SYSTEM,
} from "./protocol";
import type { ContentBlock } from "../types";

function block(kind: ContentBlock["kind"], text: string, markup: ContentBlock["markup"] = []): ContentBlock {
  return {
    id: `00000000-0000-0000-0000-${Math.random().toString(16).slice(2, 14)}`,
    kind,
    text,
    position: 0,
    markup,
  };
}

describe("buildTrainingExamples", () => {
  it("pairs thinking with following body and uses chapter title for opening", () => {
    const examples = buildTrainingExamples(
      [
        block("thinking", "意图：让读者感到怀表有秘密，但不揭晓是谁留的"),
        block("body", "林默站在窗前。雾已经漫过码头的铁索，潮声一下一下敲着船帮。"),
        block("body", "他摩挲着那只旧怀表，盖沿的铜锈蹭在指腹上，像有人刚刚摸过。"),
        block("thinking", "意图：让表盖内侧的两个字入镜，但不解释含义"),
        block("body", "他把表盖掀开一条缝。内侧刻着两个字，笔画浅得像被潮气咬过，灯火一晃，那两个字又隐回去了。"),
      ],
      false,
      "雾港来客",
    );
    expect(examples).toHaveLength(2);
    expect(examples[0].thinking).toContain("意图：让读者感到怀表有秘密");
    expect(examples[0].content).toContain("林默站在窗前。");
    expect(examples[0].instruction).toBe("写下《雾港来客》的开篇。");
    expect(examples[0].context).toBe("");
    expect(examples[0].quality).toBe("gold");
    expect(examples[1].instruction).toBe("续写下一段。");
    expect(examples[1].context).toContain("【思考】意图：让读者感到怀表有秘密");
    expect(examples[1].context).toContain("林默站在窗前。");
    expect(examples[1].context).not.toContain("表盖掀开");
  });

  it("merges consecutive thinking blocks into one beat", () => {
    const examples = buildTrainingExamples([
      block("thinking", "意图：港口第一眼就要冷，把有人在等压到段末"),
      block("thinking", "约束：只写林默所见，雾的来源本章不解释"),
      block(
        "body",
        "雾先于潮声漫进港口。石阶湿了一圈，远处有人把灯笼从帆布里掏出来，光却到不了这边。",
      ),
    ]);
    expect(examples).toHaveLength(1);
    expect(examples[0].thinking).toContain("意图：港口第一眼就要冷");
    expect(examples[0].quality).toBe("gold");
  });

  it("marks body without thinking as skip and drops it by default", () => {
    const examples = buildTrainingExamples([
      block("body", "开篇没有思考，这段正文再长也不能进默认训练集，只会被标成 skip。"),
      block("thinking", "意图：后面才补思考，让这一拍成为金标样本"),
      block(
        "body",
        "思考后的正文必须写够一个段落，潮声、雾和人影都要落到纸上，连灯笼的光都到不了这边。",
      ),
    ]);
    expect(examples[0].quality).toBe("skip");
    expect(examples[0].skipReasons).toContain("emptyThinking");
    expect(filterExamples(examples, "usable")).toHaveLength(1);
  });

  it("keeps chapter-start thinking and does not truncate long body", () => {
    const longBody = `${"雾已经漫过码头的铁索".repeat(40)}。`;
    const examples = buildTrainingExamples(
      [
        block("thinking", "意图：铺一整段冷开场，让雾先压住港口"),
        block("body", longBody),
        block("thinking", "意图：再写人影，但不让读者看清脸"),
        block("body", "远处有人把灯笼从帆布里掏出来，光却到不了这边。石阶湿了一圈。"),
      ],
      false,
      "雾港来客",
    );
    expect(examples[1].context.startsWith("【思考】意图：铺一整段冷开场")).toBe(true);
    expect(examples[1].context).toContain(longBody);
    expect(examples[1].context).not.toContain("灯笼从帆布");
  });

  it("treats story tags as labels and does not duplicate typed text", () => {
    expect(STORY_TAG_KINDS).toEqual(["人物", "伏笔", "地点", "道具", "势力", "规则"]);
    const tagged = block("thinking", "意图：让林默出场\n@人物:林默", [
      { type: "tag", id: "", kind: "人物", label: "林默", note: "" },
    ]);
    const examples = buildTrainingExamples(
      [tagged, block("body", "林默站在窗前。雾已经漫过码头的铁索，潮声一下一下敲着船帮。")],
      true,
    );
    expect(examples[0].thinking).toContain("@人物:林默");
    expect(examples[0].thinking).not.toContain("[@人物：林默]");

    const missing = block("thinking", "意图：让林默出场", [
      { type: "tag", id: "", kind: "人物", label: "林默", note: "" },
    ]);
    const appended = buildTrainingExamples(
      [missing, block("body", "林默站在窗前。雾已经漫过码头的铁索，潮声一下一下敲着船帮。")],
      true,
    );
    expect(appended[0].thinking).toContain("[@人物：林默]");
  });
});

describe("parseThinkingSlots", () => {
  it("accepts synonym labels", () => {
    const slots = parseThinkingSlots(
      "先定调\n目标：让雾先压过来\n限制：不解释来源\n写法：短句\n必须：潮声\n勿：开场独白",
    );
    expect(slots.intent).toBe("让雾先压过来");
    expect(slots.constraints).toBe("不解释来源");
    expect(slots.technique).toBe("短句");
    expect(slots.mustShow).toBe("潮声");
    expect(slots.mustNot).toBe("开场独白");
    expect(slots.notes).toBe("先定调");
  });
});

describe("serializeExamples", () => {
  const examples = buildTrainingExamples(
    [
      block("thinking", "意图：港口第一眼就要冷，把有人在等压到段末"),
      block(
        "body",
        "雾先于脚步声漫进港口。石阶湿了一圈，灯笼的光到不了这边，潮声一下一下敲着船帮上。",
      ),
    ],
    false,
    "雾港来客",
  );

  it("serializes jsonl with protocol fields", () => {
    const out = serializeExamples(examples, "jsonl");
    expect(out).toContain('"thinking":"意图：港口第一眼就要冷，把有人在等压到段末"');
    expect(out).toContain('"instruction":"写下《雾港来客》的开篇。"');
    expect(out).toContain('"quality":"gold"');
  });

  it("never uses a dummy continue-writing human prompt", () => {
    const out = serializeExamples(examples, "sharegpt");
    expect(out).toContain(WRITING_PROTOCOL_SYSTEM);
    expect(out).toContain("写下《雾港来客》的开篇。");
    expect(out).not.toContain("继续写作");
  });

  it("serializes alpaca and r1", () => {
    const alpaca = serializeExamples(examples, "alpaca");
    expect(alpaca).toContain('"input"');
    const r1 = serializeExamples(examples, "r1");
    expect(r1).toContain("<think>\n意图：港口第一眼就要冷，把有人在等压到段末\n</think>");
  });
});
