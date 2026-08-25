import type { Editor } from "@tiptap/core";
import { Extension } from "@tiptap/core";
import type { BlockKind, ContentBlock, MarkupRef } from "../types";

export type { BlockKind, ContentBlock, MarkupRef };

/** 一条训练样本：思考 + 正文 */
export interface TrainingExample {
  thinking: string;
  content: string;
}

export type ExportFormat = "jsonl" | "sharegpt" | "r1";

function newId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (char) => {
    const rand = (Math.random() * 16) | 0;
    const value = char === "x" ? rand : (rand & 0x3) | 0x8;
    return value.toString(16);
  });
}

/** 给 paragraph / thinkingBlock 挂稳定 blockId，保存时不再每次换 UUID。 */
export const BlockIdentity = Extension.create({
  name: "blockIdentity",
  addGlobalAttributes() {
    return [
      {
        types: ["paragraph", "thinkingBlock"],
        attributes: {
          blockId: {
            default: null,
            parseHTML: (element) => element.getAttribute("data-block-id"),
            renderHTML: (attributes) =>
              attributes.blockId ? { "data-block-id": attributes.blockId } : {},
          },
        },
      },
    ];
  },
});

/**
 * 把编辑器文档序列化为块序列。
 * - paragraph  -> body 块
 * - thinkingBlock -> thinking 块（含 markup 提取）
 */
export function editorToBlocks(editor: Editor): ContentBlock[] {
  const json = editor.getJSON();
  const blocks: ContentBlock[] = [];

  for (const node of json.content ?? []) {
    if (node.type === "paragraph" || node.type === "thinkingBlock") {
      const kind: BlockKind = node.type === "thinkingBlock" ? "thinking" : "body";
      let text = "";
      const markup: MarkupRef[] = [];

      const walk = (n: { type?: string; text?: string; attrs?: Record<string, unknown>; marks?: Array<{ type?: string; attrs?: Record<string, unknown> }> }) => {
        if (n.type === "text" && typeof n.text === "string") {
          text += n.text;
          for (const mark of n.marks ?? []) {
            if (mark.type === "markupRef" && mark.attrs) {
              const ref = markupFromAttrs(mark.attrs);
              if (ref) markup.push(ref);
            }
          }
        } else if (n.type === "markupRef" && n.attrs) {
          const ref = markupFromAttrs(n.attrs);
          if (ref) markup.push(ref);
          if (typeof n.text === "string") text += n.text;
        }
        for (const child of (n as { content?: typeof n[] }).content ?? []) {
          walk(child);
        }
      };
      walk(node as never);

      const attrs = (node.attrs ?? {}) as Record<string, unknown>;
      blocks.push({
        id: String(attrs.blockId ?? "") || newId(),
        kind,
        text: text.trim(),
        position: blocks.length,
        markup,
      });
    }
  }
  return blocks.filter((b) => b.text.length > 0 || b.markup.length > 0);
}

function markupToAttrs(ref: MarkupRef): Record<string, string> {
  switch (ref.type) {
    case "task":
      return { kind: "task", id: ref.id, label: ref.label, status: ref.status };
    case "setting":
      return {
        kind: "setting",
        entityPath: ref.entityPath,
        field: ref.field,
        value: ref.value,
      };
    case "custom":
      return { kind: "custom", tag: ref.tag, body: ref.body };
  }
}

function inlineContent(block: ContentBlock): object[] {
  if (!block.text && block.markup.length === 0) return [];
  if (block.markup.length === 0) {
    return [{ type: "text", text: block.text }];
  }
  return [
    {
      type: "text",
      text: block.text || markupLabel(block.markup[0]),
      marks: block.markup.map((ref) => ({ type: "markupRef", attrs: markupToAttrs(ref) })),
    },
  ];
}

/** 把块序列还原成 Tiptap 文档，供打开章节时注入。 */
export function blocksToDoc(blocks: ContentBlock[]): Record<string, unknown> {
  if (blocks.length === 0) {
    return { type: "doc", content: [{ type: "paragraph" }] };
  }
  return {
    type: "doc",
    content: blocks.map((block) => ({
      type: block.kind === "thinking" ? "thinkingBlock" : "paragraph",
      attrs: { blockId: block.id },
      content: inlineContent(block),
    })),
  };
}

function markupFromAttrs(attrs: Record<string, unknown>): MarkupRef | null {
  const kind = String(attrs.kind ?? "");
  switch (kind) {
    case "task": {
      const id = String(attrs.id ?? "");
      const label = String(attrs.label ?? "");
      const status = String(attrs.status ?? "todo");
      return id || label ? { type: "task", id, label, status } : null;
    }
    case "setting": {
      const entityPath = String(attrs.entityPath ?? "");
      const field = String(attrs.field ?? "");
      const value = String(attrs.value ?? "");
      return entityPath ? { type: "setting", entityPath, field, value } : null;
    }
    case "custom": {
      const tag = String(attrs.tag ?? "");
      const body = String(attrs.body ?? "");
      return tag ? { type: "custom", tag, body } : null;
    }
    default:
      return null;
  }
}

/** 标记引用的可读标签，与 Rust 端 label() 一致 */
export function markupLabel(ref: MarkupRef): string {
  switch (ref.type) {
    case "task":
      return `任务[${ref.status}]: ${ref.label}`;
    case "setting":
      return `设定: ${ref.entityPath}.${ref.field}`;
    case "custom":
      return `@${ref.tag}: ${ref.body}`;
  }
}

/**
 * 按「思考 -> 正文」配对构建训练样本。
 * 连续 thinking 块合并为同一段思考；连续 body 块累积为同一段正文，
 * 直到下一个 thinking 出现时结算上一条样本。与 Rust 端 build_training_examples 一致。
 */
export function buildTrainingExamples(
  blocks: ContentBlock[],
  includeMarkup = false,
): TrainingExample[] {
  const examples: TrainingExample[] = [];
  let thinking = "";
  let content = "";

  for (const block of blocks) {
    if (block.kind === "thinking") {
      // 新的思考段：先结算上一条思考 + 正文对（若有未结算正文）
      if (content) {
        examples.push({ thinking, content });
        thinking = "";
        content = "";
      }
      if (thinking) thinking += "\n";
      thinking += block.text;
      if (includeMarkup) {
        for (const ref of block.markup) {
          thinking += `\n[${markupLabel(ref)}]`;
        }
      }
    } else {
      if (content) content += "\n";
      content += block.text;
    }
  }
  if (content) {
    examples.push({ thinking, content });
  }
  return examples;
}

function r1Turn(thinking: string, content: string): string {
  const t = thinking.trim();
  if (t) {
    return `<think>\n${t}\n</think>\n\n${content}`;
  }
  return content;
}

/**
 * 序列化为三种训练格式，与 Rust 端 serialize_examples 一致：
 * - jsonl: 每行 {"thinking": "...", "content": "..."}
 * - sharegpt: [{"conversations": [{"from": "human", ...}, {"from": "assistant", ...}]}]
 * - r1: "<think>...</think>\n\n正文" 每条一行（JSON 转义）
 */
export function serializeExamples(examples: TrainingExample[], format: ExportFormat): string {
  switch (format) {
    case "jsonl":
      return examples
        .map((e) => JSON.stringify({ thinking: e.thinking, content: e.content }))
        .join("\n");
    case "sharegpt":
      return JSON.stringify(
        examples.map((e) => ({
          conversations: [
            { from: "human", value: "继续写作" },
            { from: "assistant", value: r1Turn(e.thinking, e.content) },
          ],
        })),
      );
    case "r1":
      return examples.map((e) => r1Turn(e.thinking, e.content)).join("\n");
  }
}

export function formatFilename(format: ExportFormat): string {
  const stamp = new Date().toISOString().slice(0, 10);
  switch (format) {
    case "jsonl":
      return `training-${stamp}.jsonl`;
    case "sharegpt":
      return `training-${stamp}.json`;
    case "r1":
      return `training-${stamp}.txt`;
  }
}

export function downloadText(filename: string, content: string): void {
  const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  document.body.removeChild(anchor);
  URL.revokeObjectURL(url);
}
