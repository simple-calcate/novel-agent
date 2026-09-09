import { Node, InputRule, mergeAttributes } from "@tiptap/core";
import type { Editor } from "@tiptap/core";
import {
  NodeViewWrapper,
  NodeViewContent,
  ReactNodeViewRenderer,
  ReactNodeViewProps,
} from "@tiptap/react";
import { Brain, ChevronDown, ChevronRight } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { THINKING_STARTER } from "./guide";
import { useStoryCatalog } from "./StoryCatalog";
import { listTagCandidates } from "../structure/tagCandidates";
import { FloatingMenu } from "../components/FloatingMenu";

/**
 * 思考块：作者决策层，读者看不到。协议见 docs/writing-protocol.md
 * - 空行行首 Tab 切换思考/正文（拍的边界）
 * - 行首 `>> ` 把当前段落转为思考块
 * - 思考块内 Enter 延续同一拍思考
 * - 思考块行首 `<< ` 或 Mod-Enter 结束思考，开始本拍正文
 * - 可折叠为单行摘要
 */
export const ThinkingBlock = Node.create({
  name: "thinkingBlock",

  group: "block",

  content: "inline*",

  defining: true,

  addAttributes() {
    return {
      collapsed: {
        default: false,
        parseHTML: (element) => element.getAttribute("data-collapsed") === "true",
        renderHTML: (attributes) => ({
          "data-collapsed": attributes.collapsed ? "true" : "false",
        }),
      },
    };
  },

  parseHTML() {
    return [{ tag: "div.thinking-block" }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      "div",
      mergeAttributes(HTMLAttributes, {
        class: "thinking-block",
        "data-thinking": "",
      }),
      0,
    ];
  },

  addKeyboardShortcuts() {
    return {
      // 思考块内 Enter：延续思考（splitBlock 复制节点类型与 attrs）
      Enter: ({ editor }) => {
        const { $from } = editor.state.selection;
        if ($from.parent.type.name === "thinkingBlock") {
          return editor.chain().splitBlock().run();
        }
        return false;
      },
      // Mod-Enter：退出思考，转到新正文段落
      "Mod-Enter": ({ editor }) => {
        const { $from } = editor.state.selection;
        if ($from.parent.type.name === "thinkingBlock") {
          return editor
            .chain()
            .setParagraph()
            .insertContentAt(editor.state.selection.to, { type: "paragraph" })
            .focus(editor.state.selection.to + 1)
            .run();
        }
        return false;
      },
      // Mod-Shift-B：任意位置切换正文/思考
      "Mod-Shift-B": ({ editor }) => {
        const { $from } = editor.state.selection;
        const type = editor.state.schema.nodes.thinkingBlock;
        if ($from.parent.type === type) {
          return editor.chain().setParagraph().run();
        }
        const empty = $from.parent.textContent.length === 0;
        const chain = editor.chain().setNode(type);
        if (empty) chain.insertContent(THINKING_STARTER);
        return chain.run();
      },
    };
  },

  addInputRules() {
    return [
      // 行首 `>> ` -> 转思考块
      new InputRule({
        find: /^\s*>>\s$/,
        handler: ({ state, range, chain }) => {
          const { $from } = state.selection;
          if ($from.parent.type.name === "thinkingBlock") return;
          chain()
            .deleteRange(range)
            .setNode(state.schema.nodes.thinkingBlock)
            .insertContent(THINKING_STARTER)
            .run();
        },
      }),
      // 思考块行首 `<< ` -> 退出思考
      new InputRule({
        find: /^\s*<<\s$/,
        handler: ({ state, range, chain }) => {
          const { $from } = state.selection;
          if ($from.parent.type.name !== "thinkingBlock") return;
          chain()
            .deleteRange(range)
            .setParagraph()
            .run();
        },
      }),
    ];
  },

  addNodeView() {
    return ReactNodeViewRenderer(ThinkingBlockView);
  },
});

function ThinkingBlockView({ node, updateAttributes }: ReactNodeViewProps) {
  const collapsed = node.attrs.collapsed === true;
  const isEmpty = node.content.size === 0;
  const firstLine = (node.textContent || "思考…").split("\n")[0];
  const summary = firstLine.length > 40 ? `${firstLine.slice(0, 40)}…` : firstLine;

  return (
    <NodeViewWrapper className="thinking-block" data-thinking="">
      <button
        className="thinking-toggle"
        contentEditable={false}
        onClick={() => updateAttributes({ collapsed: !collapsed })}
        title={collapsed ? "展开思考" : "折叠思考"}
      >
        {collapsed ? <ChevronRight size={12} /> : <ChevronDown size={12} />}
      </button>
      <span className="thinking-icon" contentEditable={false}>
        <Brain size={12} />
      </span>
      {collapsed ? (
        <span className="thinking-summary" contentEditable={false}>
          {summary}
          <span className="thinking-summary-hint">· 思考已折叠</span>
        </span>
      ) : (
        <span className="thinking-content-wrap">
          {isEmpty && (
            <span className="thinking-placeholder" contentEditable={false}>
              意图：这一拍要完成什么（读者看不到）
            </span>
          )}
          <NodeViewContent className="thinking-content" />
        </span>
      )}
    </NodeViewWrapper>
  );
}

const MARKUP_ATTRS = {
  kind: { default: "tag" },
  id: { default: "" },
  label: { default: "" },
  status: { default: "todo" },
  entityPath: { default: "" },
  field: { default: "" },
  value: { default: "" },
  tagKind: { default: "" },
  tag: { default: "" },
  body: { default: "" },
  note: { default: "" },
};

/** 写作标签：独立行内块，名字在块里输入，不等于正史库条目。 */
export const MarkupRef = Node.create({
  name: "markupRef",

  group: "inline",

  inline: true,

  atom: true,

  selectable: true,

  draggable: false,

  priority: 1000,

  addAttributes() {
    return MARKUP_ATTRS;
  },

  parseHTML() {
    return [
      {
        tag: "span[data-markup-ref]",
        getAttrs: (node) => {
          const el = node as HTMLElement;
          const tagKind = el.getAttribute("data-tag-kind") ?? "";
          const kind = el.getAttribute("data-markup-ref") ?? "tag";
          const labelled = el.getAttribute("data-label");
          const fallback = (el.textContent ?? "").replace(/^@[^：:]+[：:]/, "").trim();
          return {
            kind,
            tagKind,
            label: labelled ?? fallback,
            id: el.getAttribute("data-id") ?? "",
            note: el.getAttribute("data-note") ?? "",
            status: el.getAttribute("data-status") ?? "todo",
            entityPath: el.getAttribute("data-entity-path") ?? "",
            field: el.getAttribute("data-field") ?? "",
            value: el.getAttribute("data-value") ?? "",
            tag: el.getAttribute("data-tag") ?? "",
            body: el.getAttribute("data-body") ?? "",
          };
        },
      },
    ];
  },

  renderHTML({ node, HTMLAttributes }) {
    const tagKind = String(node.attrs.tagKind ?? "");
    const label = String(node.attrs.label ?? "");
    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-markup-ref": node.attrs.kind,
        "data-tag-kind": tagKind,
        "data-label": label,
        class: `markup-chip markup-ref markup-${node.attrs.kind}`,
      }),
      tagKind ? `@${tagKind}：${label}` : `@${label}`,
    ];
  },

  addKeyboardShortcuts() {
    return {
      ArrowLeft: ({ editor }) => enterMarkupChip(editor, "left"),
      ArrowRight: ({ editor }) => enterMarkupChip(editor, "right"),
    };
  },

  addNodeView() {
    return ReactNodeViewRenderer(MarkupChipView, {
      as: "span",
      stopEvent: ({ event }) => event.target instanceof HTMLInputElement,
    });
  },
});

/** 光标贴在标签外侧时，左右键进入内部输入框。 */
export function enterMarkupChip(editor: Editor, direction: "left" | "right"): boolean {
  const { selection } = editor.state;
  const selectedNode = "node" in selection ? (selection as { node?: { type: { name: string } }; from: number }).node : null;
  if (selectedNode?.type.name === "markupRef") {
    return focusMarkupChipInput(editor, selection.from, direction === "left" ? "end" : "start");
  }
  if (!selection.empty) return false;
  const $from = selection.$from;
  if (direction === "left") {
    const node = $from.nodeBefore;
    if (node?.type.name !== "markupRef") return false;
    return focusMarkupChipInput(editor, $from.pos - node.nodeSize, "end");
  }
  const node = $from.nodeAfter;
  if (node?.type.name !== "markupRef") return false;
  return focusMarkupChipInput(editor, $from.pos, "start");
}

export function focusMarkupChipInput(editor: Editor, pos: number, edge: "start" | "end"): boolean {
  const input = findChipInput(asDom(editor.view.nodeDOM(pos))) ?? findChipInput(domAt(editor, pos));
  if (!input) return false;
  input.focus();
  const at = edge === "start" ? 0 : input.value.length;
  input.setSelectionRange(at, at);
  return true;
}

function asDom(value: unknown): globalThis.Node | null {
  return value instanceof globalThis.Node ? value : null;
}

function domAt(editor: Editor, pos: number): globalThis.Node | null {
  try {
    return asDom(editor.view.domAtPos(Math.min(pos + 1, editor.state.doc.content.size)).node);
  } catch {
    return null;
  }
}

function findChipInput(dom: globalThis.Node | null | undefined): HTMLInputElement | null {
  if (!dom) return null;
  if (dom instanceof HTMLInputElement) return dom;
  if (dom instanceof HTMLElement) {
    const chip = dom.classList.contains("markup-chip")
      ? dom
      : (dom.closest(".markup-chip") ?? dom.querySelector(".markup-chip"));
    const input = chip?.querySelector("input.markup-chip-input");
    return input instanceof HTMLInputElement ? input : null;
  }
  return findChipInput(dom.parentElement);
}

function posOf(getPos: () => number | undefined): number | null {
  const pos = getPos();
  return typeof pos === "number" ? pos : null;
}

function focusAfterChip(editor: Editor, getPos: () => number | undefined, nodeSize: number) {
  const pos = posOf(getPos);
  if (pos == null) return;
  editor.chain().setTextSelection(pos + nodeSize).focus().run();
}

function MarkupChipView({ node, updateAttributes, getPos, editor, selected }: ReactNodeViewProps) {
  const tagKind = String(node.attrs.tagKind || "标签");
  const [value, setValue] = useState(String(node.attrs.label || ""));
  const [open, setOpen] = useState(true);
  const [active, setActive] = useState(0);
  const [caret, setCaret] = useState<{ top: number; left: number; bottom: number } | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const composing = useRef(false);
  const blurTimer = useRef<ReturnType<typeof setTimeout>>();
  const { entries, nearby } = useStoryCatalog();
  const candidates = listTagCandidates(entries, tagKind, value, nearby);

  const measure = () => {
    const el = inputRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    setCaret({ top: rect.top, left: rect.left, bottom: rect.bottom });
  };

  useEffect(() => {
    if (!String(node.attrs.label || "")) {
      inputRef.current?.focus();
    }
    measure();
    // 只在插入空标签时抢焦点，避免输入过程中重渲染把光标拽回去
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!selected) return;
    const input = inputRef.current;
    if (!input || document.activeElement === input) return;
    input.focus();
    input.setSelectionRange(input.value.length, input.value.length);
  }, [selected]);

  useEffect(() => {
    setActive(0);
    measure();
  }, [value, tagKind]);

  useEffect(() => {
    if (!open) return;
    const onScroll = () => measure();
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onScroll);
    return () => {
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onScroll);
    };
  }, [open]);

  const commit = (next: string) => {
    if (next !== String(node.attrs.label || "")) {
      updateAttributes({ label: next });
    }
  };

  const pick = (name: string) => {
    setValue(name);
    commit(name);
    setOpen(false);
    focusAfterChip(editor, getPos, node.nodeSize);
  };

  return (
    <NodeViewWrapper
      as="span"
      className={`markup-chip markup-ref markup-${node.attrs.kind}${selected ? " is-selected" : ""}`}
      data-markup-ref={node.attrs.kind}
      data-tag-kind={tagKind}
      contentEditable={false}
      onClick={() => inputRef.current?.focus()}
    >
      <span className="markup-chip-kind">@{tagKind}</span>
      <span className="markup-chip-colon">：</span>
      <input
        ref={inputRef}
        className="markup-chip-input"
        value={value}
        size={Math.max(2, value.length + 1)}
        placeholder="名字"
        spellCheck={false}
        aria-label={`${tagKind}名称`}
        onMouseDown={(event) => event.stopPropagation()}
        onFocus={() => {
          if (blurTimer.current) clearTimeout(blurTimer.current);
          setOpen(true);
          measure();
        }}
        onCompositionStart={() => {
          composing.current = true;
        }}
        onCompositionEnd={(event) => {
          composing.current = false;
          const next = event.currentTarget.value;
          setValue(next);
          commit(next);
        }}
        onChange={(event) => {
          const next = event.target.value;
          setValue(next);
          setOpen(true);
          if (!composing.current) commit(next);
        }}
        onKeyDown={(event) => {
          event.stopPropagation();
          if (composing.current) return;
          const caretPos = event.currentTarget.selectionStart ?? 0;
          const end = event.currentTarget.value.length;
          if (open && candidates.length > 0 && event.key === "ArrowDown") {
            event.preventDefault();
            setActive((index) => (index + 1) % candidates.length);
            return;
          }
          if (open && candidates.length > 0 && event.key === "ArrowUp") {
            event.preventDefault();
            setActive((index) => (index - 1 + candidates.length) % candidates.length);
            return;
          }
          if (event.key === "Enter" || event.key === "Tab") {
            event.preventDefault();
            const hit = open ? candidates[active] : undefined;
            if (hit) {
              pick(hit.name);
              return;
            }
            commit(value);
            focusAfterChip(editor, getPos, node.nodeSize);
            return;
          }
          if (event.key === "Escape") {
            event.preventDefault();
            commit(value);
            focusAfterChip(editor, getPos, node.nodeSize);
            return;
          }
          if (event.key === "Backspace" && value === "") {
            event.preventDefault();
            const pos = posOf(getPos);
            if (pos == null) return;
            editor.chain().deleteRange({ from: pos, to: pos + node.nodeSize }).focus().run();
            return;
          }
          if (event.key === "ArrowLeft" && caretPos === 0) {
            event.preventDefault();
            const pos = posOf(getPos);
            if (pos == null) return;
            editor.chain().setTextSelection(pos).focus().run();
            return;
          }
          if (event.key === "ArrowRight" && caretPos === end) {
            event.preventDefault();
            focusAfterChip(editor, getPos, node.nodeSize);
          }
        }}
        onBlur={() => {
          commit(value);
          blurTimer.current = setTimeout(() => setOpen(false), 120);
        }}
      />
      {open && caret && candidates.length > 0 && (
        <FloatingMenu
          caret={caret}
          itemCount={candidates.length}
          onMouseDown={(event) => event.preventDefault()}
        >
          {candidates.map((item, index) => (
            <button
              key={item.id}
              className={`mention-item${index === active ? " active" : ""}`}
              onClick={() => pick(item.name)}
            >
              <span className="mention-meta">
                <span className="mention-label">{item.name}</span>
                <span className="mention-desc">
                  {item.matchedAlias ? `别名 ${item.matchedAlias}` : item.summary}
                </span>
              </span>
            </button>
          ))}
        </FloatingMenu>
      )}
    </NodeViewWrapper>
  );
}
