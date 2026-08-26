import type { Editor } from "@tiptap/core";
import { Extension } from "@tiptap/core";
import type { BlockKind, ContentBlock, MarkupRef } from "../types";

export type { BlockKind, ContentBlock, MarkupRef };
export {
  buildTrainingExamples,
  filterExamples,
  formatFilename,
  qualityCounts,
  serializeExamples,
  type ExportFormat,
  type TrainingExample,
} from "./protocol";

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

/** 标记引用的可读标签（编辑器展示）。训练导出用 Rust 口径的摘要，见 protocol.ts。 */
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
