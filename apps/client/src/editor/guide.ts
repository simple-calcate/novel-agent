/** 作者在编辑器里看到的写作协议。导出分级另见 protocol.ts。 */

export type WriterMode = "body" | "thinking";

export const THINKING_STARTER = "意图：";

export const SLOT_PROMPTS: Array<{ insert: string; label: string; hint: string }> = [
  { insert: "意图：", label: "这段要干什么", hint: "只写一件事" },
  { insert: "约束：", label: "现在还不能写", hint: "别剧透、谁还不知道" },
  { insert: "手法：", label: "怎么写", hint: "先写景、压钩子、短句…" },
  { insert: "兑现：", label: "正文里必须出现", hint: "物件、动作、一句话" },
  { insert: "禁止：", label: "这段别写", hint: "OOC、说明文、重复" },
];

export function writerModeFromParent(parentType: string): WriterMode {
  return parentType === "thinkingBlock" ? "thinking" : "body";
}

export function guideCopy(opts: {
  mode: WriterMode;
  missingThinkingBeats: number;
}): { title: string; body: string } {
  if (opts.mode === "thinking") {
    return {
      title: "思考 · 读者看不到",
      body: "写一句这段要干什么。写完空行按 Tab，回去写读者看到的句子。",
    };
  }
  if (opts.missingThinkingBeats > 0) {
    return {
      title: "正文 · 读者会看到",
      body: `有 ${opts.missingThinkingBeats} 段正文前面没有思考，导出时带不走。空行按 Tab，先写「这段要干什么」。`,
    };
  }
  return {
    title: "正文 · 读者会看到",
    body: "写小说。下一段有讲究时，空行按 Tab 先写思考，再回来写。",
  };
}

export function countMissingThinking(
  examples: Array<{ skipReasons: string[] }>,
): number {
  return examples.filter((example) => example.skipReasons.includes("emptyThinking")).length;
}

export function authorExportSummary(kept: number, missingThinking: number): string {
  if (kept === 0 && missingThinking === 0) {
    return "这一章还没有可以带走的段落。请先写思考，再写正文。";
  }
  if (kept === 0) {
    return `有 ${missingThinking} 段正文，但前面都没有思考，所以没有导出。空行按 Tab，补一句这段要干什么。`;
  }
  if (missingThinking === 0) {
    return `已导出 ${kept} 段完整的「思考 + 正文」。`;
  }
  return `已导出 ${kept} 段。另有 ${missingThinking} 段只有正文、没有思考，没有导出。`;
}
