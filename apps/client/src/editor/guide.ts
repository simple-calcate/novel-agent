/** 作者在编辑器里看到的写作协议。导出分级另见 protocol.ts。 */

import { t } from "../i18n";

export type WriterMode = "body" | "thinking";

export const THINKING_STARTER = "意图：";

/** 槽位写入正文的协议前缀，不随界面语言改变。 */
export const SLOT_INSERTS = ["意图：", "约束：", "手法：", "兑现：", "禁止："] as const;

export function slotPrompts(): Array<{ insert: string; label: string; hint: string }> {
  return [
    { insert: "意图：", label: t("guide.slotIntent"), hint: t("guide.slotIntentHint") },
    { insert: "约束：", label: t("guide.slotConstraint"), hint: t("guide.slotConstraintHint") },
    { insert: "手法：", label: t("guide.slotTechnique"), hint: t("guide.slotTechniqueHint") },
    { insert: "兑现：", label: t("guide.slotMustShow"), hint: t("guide.slotMustShowHint") },
    { insert: "禁止：", label: t("guide.slotMustNot"), hint: t("guide.slotMustNotHint") },
  ];
}

export function writerModeFromParent(parentType: string): WriterMode {
  return parentType === "thinkingBlock" ? "thinking" : "body";
}

export function guideCopy(opts: {
  mode: WriterMode;
  missingThinkingBeats: number;
}): { title: string; body: string } {
  if (opts.mode === "thinking") {
    return {
      title: t("guide.thinkingTitle"),
      body: t("guide.thinkingBody"),
    };
  }
  if (opts.missingThinkingBeats > 0) {
    return {
      title: t("guide.bodyTitle"),
      body: t("guide.bodyMissing", { count: opts.missingThinkingBeats }),
    };
  }
  return {
    title: t("guide.bodyTitle"),
    body: t("guide.bodyReady"),
  };
}

export function countMissingThinking(
  examples: Array<{ skipReasons: string[] }>,
): number {
  return examples.filter((example) => example.skipReasons.includes("emptyThinking")).length;
}

export function authorExportSummary(kept: number, missingThinking: number): string {
  if (kept === 0 && missingThinking === 0) {
    return t("guide.exportEmpty");
  }
  if (kept === 0) {
    return t("guide.exportNoneKept", { missing: missingThinking });
  }
  if (missingThinking === 0) {
    return t("guide.exportAll", { kept });
  }
  return t("guide.exportPartial", { kept, missing: missingThinking });
}
