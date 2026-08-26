/**
 * 墨枢写作协议 v1 的前端镜像。权威实现：`crates/domain/src/protocol.rs`
 * 规范：`docs/writing-protocol.md`
 */

import type { ContentBlock, MarkupRef } from "../types";

export const WRITING_PROTOCOL_VERSION = 1 as const;

export const WRITING_PROTOCOL_SYSTEM =
  "你按墨枢写作协议写网文：先思考本拍决策，再写读者可见正文。思考不是人物内心独白，正文不含作者备注。";

const CONTEXT_CHAR_LIMIT = 600;
const GOLD_THINKING_MIN = 16;
const GOLD_BODY_MIN = 40;
const USABLE_THINKING_MIN = 8;
const USABLE_BODY_MIN = 24;
const COPY_DETECT_MIN = 16;

export type ExampleQuality = "skip" | "usable" | "gold";
export type ExportFormat = "jsonl" | "sharegpt" | "alpaca" | "r1";

const QUALITY_RANK: Record<ExampleQuality, number> = {
  skip: 0,
  usable: 1,
  gold: 2,
};

export interface ThinkingSlots {
  intent?: string;
  constraints?: string;
  technique?: string;
  mustShow?: string;
  mustNot?: string;
  notes: string;
}

export interface TrainingExample {
  protocolVersion: number;
  beatIndex: number;
  quality: ExampleQuality;
  skipReasons: string[];
  context: string;
  instruction: string;
  thinking: string;
  content: string;
  slots: ThinkingSlots;
}

export interface QualityCounts {
  gold: number;
  usable: number;
  skip: number;
}

type SlotKind = "intent" | "constraints" | "technique" | "mustShow" | "mustNot";

function charCount(text: string): number {
  return [...text].length;
}

function takeContextExcerpt(preceding: string): string {
  const trimmed = preceding.trim();
  if (!trimmed) return "";
  const chars = [...trimmed];
  if (chars.length <= CONTEXT_CHAR_LIMIT) return trimmed;
  const slice = chars.slice(-CONTEXT_CHAR_LIMIT).join("");
  const newline = slice.indexOf("\n");
  if (newline >= 0) {
    const after = slice.slice(newline).trim();
    if (after) return after;
  }
  return slice.trim();
}

function beatInstruction(context: string, chapterTitle?: string): string {
  if (!context) {
    const title = chapterTitle?.trim();
    return title ? `写下《${title}》的开篇。` : "写下本章开篇。";
  }
  return "续写下一段。";
}

function normalizeLabel(label: string): string {
  return label.trim().replace(/^[[【\s]+/, "").replace(/[\]】\s]+$/, "");
}

function splitSlotLine(line: string): { kind: SlotKind; rest: string } | null {
  const colon = line.includes("：") ? "：" : line.includes(":") ? ":" : null;
  if (!colon) return null;
  const [label, ...restParts] = line.split(colon);
  const rest = restParts.join(colon).trim();
  const normalized = normalizeLabel(label ?? "");
  const kind: SlotKind | null =
    ["意图", "目标", "目的"].includes(normalized)
      ? "intent"
      : ["约束", "限制", "正史"].includes(normalized)
        ? "constraints"
        : ["手法", "写法", "节奏"].includes(normalized)
          ? "technique"
          : ["兑现", "必须", "出场"].includes(normalized)
            ? "mustShow"
            : ["禁止", "勿", "不能"].includes(normalized)
              ? "mustNot"
              : null;
  return kind ? { kind, rest } : null;
}

function appendSlot(slots: ThinkingSlots, kind: SlotKind, text: string): void {
  if (!text) return;
  const current = slots[kind];
  slots[kind] = current ? `${current}\n${text}` : text;
}

export function parseThinkingSlots(thinking: string): ThinkingSlots {
  const slots: ThinkingSlots = { notes: "" };
  let current: SlotKind | null = null;
  const notes: string[] = [];

  for (const raw of thinking.split("\n")) {
    const line = raw.trim();
    if (!line) continue;
    const parsed = splitSlotLine(line);
    if (parsed) {
      current = parsed.kind;
      appendSlot(slots, parsed.kind, parsed.rest);
    } else if (current) {
      appendSlot(slots, current, line);
    } else {
      notes.push(line);
    }
  }
  slots.notes = notes.join("\n");
  return slots;
}

function isCopiedAcrossLayers(thinking: string, content: string): boolean {
  const t = thinking.trim();
  const c = content.trim();
  if (charCount(t) < COPY_DETECT_MIN && charCount(c) < COPY_DETECT_MIN) return false;
  if (charCount(t) >= COPY_DETECT_MIN && c.includes(t)) return true;
  if (charCount(c) >= COPY_DETECT_MIN && t.includes(c)) return true;
  return false;
}

function bodyLooksLikeNotes(content: string): boolean {
  const lower = content.toLowerCase();
  if (lower.includes("todo") || lower.includes("fixme")) return true;
  if (content.includes("（作者") || content.includes("(作者") || content.includes("【作者")) {
    return true;
  }
  if (content.includes("此处待")) return true;
  return content.split("\n").some((line) => {
    const trimmed = line.trim();
    return (
      trimmed.startsWith(">> ") ||
      trimmed.startsWith("意图：") ||
      trimmed.startsWith("意图:") ||
      trimmed.startsWith("约束：") ||
      trimmed.startsWith("约束:") ||
      trimmed.startsWith("伏笔：") ||
      trimmed.startsWith("伏笔:")
    );
  });
}

export function gradeExample(
  thinking: string,
  content: string,
  slots: ThinkingSlots,
): { quality: ExampleQuality; skipReasons: string[] } {
  const thinkingLen = charCount(thinking);
  const contentLen = charCount(content);
  const skipReasons: string[] = [];

  if (!thinking) skipReasons.push("emptyThinking");
  else if (thinkingLen < USABLE_THINKING_MIN) skipReasons.push("thinkingTooShort");
  if (contentLen < USABLE_BODY_MIN) skipReasons.push("bodyTooShort");
  if (isCopiedAcrossLayers(thinking, content)) skipReasons.push("thinkingCopiedToBody");
  if (bodyLooksLikeNotes(content)) skipReasons.push("bodyLooksLikeNotes");

  if (skipReasons.length > 0) return { quality: "skip", skipReasons };

  const gold =
    Boolean(slots.intent) && thinkingLen >= GOLD_THINKING_MIN && contentLen >= GOLD_BODY_MIN;
  return { quality: gold ? "gold" : "usable", skipReasons: [] };
}

function humanPrompt(example: Pick<TrainingExample, "context" | "instruction">): string {
  return example.context ? `${example.context}\n\n${example.instruction}` : example.instruction;
}

export function assistantR1(example: Pick<TrainingExample, "thinking" | "content">): string {
  const thinking = example.thinking.trim();
  return thinking ? `<think>\n${thinking}\n</think>\n\n${example.content}` : example.content;
}

function finishExample(
  thinking: string,
  content: string,
  precedingBody: string,
  chapterTitle: string | undefined,
  beatIndex: number,
): TrainingExample {
  const trimmedThinking = thinking.trim();
  const trimmedContent = content.trim();
  const slots = parseThinkingSlots(trimmedThinking);
  const { quality, skipReasons } = gradeExample(trimmedThinking, trimmedContent, slots);
  const context = takeContextExcerpt(precedingBody);
  return {
    protocolVersion: WRITING_PROTOCOL_VERSION,
    beatIndex,
    quality,
    skipReasons,
    context,
    instruction: beatInstruction(context, chapterTitle),
    thinking: trimmedThinking,
    content: trimmedContent,
    slots,
  };
}

function markupSummary(ref: MarkupRef): string {
  switch (ref.type) {
    case "task":
      return `任务[${ref.status}]: ${ref.label}`;
    case "setting":
      return `设定 ${ref.entityPath}.${ref.field} = ${ref.value}`;
    case "custom":
      return `标记 ${ref.tag}: ${ref.body}`;
  }
}

function appendMarkup(thinking: string, markup: MarkupRef[]): string {
  if (markup.length === 0) return thinking;
  const extra = markup.map((ref) => `[${markupSummary(ref)}]`).join("\n");
  return thinking ? `${thinking}\n${extra}` : extra;
}

/**
 * 按「思考 -> 正文」切拍。连续 thinking 合并；连续 body 累积到下一拍思考出现。
 * 与 Rust `build_training_examples_from_blocks` 一致。
 */
export function buildTrainingExamples(
  blocks: ContentBlock[],
  includeMarkup = false,
  chapterTitle?: string,
): TrainingExample[] {
  const examples: TrainingExample[] = [];
  let thinking = "";
  let content = "";
  let precedingBody = "";

  const flush = () => {
    examples.push(
      finishExample(thinking, content, precedingBody, chapterTitle, examples.length),
    );
    precedingBody = precedingBody ? `${precedingBody}\n${content}` : content;
    thinking = "";
    content = "";
  };

  for (const block of blocks) {
    if (block.kind === "thinking") {
      if (content) flush();
      thinking = thinking
        ? `${thinking}\n${block.text}`
        : block.text;
      if (includeMarkup) thinking = appendMarkup(thinking, block.markup);
    } else {
      content = content ? `${content}\n${block.text}` : block.text;
    }
  }
  if (content) flush();
  return examples;
}

export function filterExamples(
  examples: TrainingExample[],
  minQuality: ExampleQuality = "usable",
): TrainingExample[] {
  const min = QUALITY_RANK[minQuality];
  return examples.filter((example) => QUALITY_RANK[example.quality] >= min);
}

export function qualityCounts(examples: TrainingExample[]): QualityCounts {
  const counts: QualityCounts = { gold: 0, usable: 0, skip: 0 };
  for (const example of examples) counts[example.quality] += 1;
  return counts;
}

export function serializeExamples(examples: TrainingExample[], format: ExportFormat): string {
  switch (format) {
    case "jsonl":
      return examples.map((example) => JSON.stringify(example)).join("\n");
    case "sharegpt":
      return JSON.stringify(
        examples.map((example) => ({
          conversations: [
            { from: "system", value: WRITING_PROTOCOL_SYSTEM },
            { from: "human", value: humanPrompt(example) },
            { from: "assistant", value: assistantR1(example) },
          ],
        })),
      );
    case "alpaca":
      return JSON.stringify(
        examples.map((example) => ({
          instruction: WRITING_PROTOCOL_SYSTEM,
          input: humanPrompt(example),
          output: assistantR1(example),
        })),
      );
    case "r1":
      return examples.map(assistantR1).join("\n\n---\n\n");
  }
}

export function formatFilename(format: ExportFormat): string {
  const stamp = new Date().toISOString().slice(0, 10);
  switch (format) {
    case "jsonl":
      return `training-${stamp}.jsonl`;
    case "sharegpt":
    case "alpaca":
      return `training-${stamp}.json`;
    case "r1":
      return `training-${stamp}.txt`;
  }
}
