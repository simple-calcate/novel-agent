import { Node, Mark, InputRule, mergeAttributes } from "@tiptap/core";
import {
  NodeViewWrapper,
  NodeViewContent,
  ReactNodeViewRenderer,
  ReactNodeViewProps,
} from "@tiptap/react";
import { Brain, ChevronDown, ChevronRight } from "lucide-react";

/**
 * 思考块：模拟 AI 的 thinking 输出。
 * - 行首输入 `>> ` 把当前段落转为思考块
 * - 思考块内 Enter 延续思考（多行 = 多个连续 thinking 块）
 * - 思考块行首输入 `<< ` 或按 Mod-Enter 退出，回到正文
 * - 可折叠为单行摘要
 */
export const ThinkingBlock = Node.create({
  name: "thinkingBlock",

  group: "block",

  content: "text*",

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
        return editor.chain().setNode(type).run();
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
        <NodeViewContent className="thinking-content" />
      )}
    </NodeViewWrapper>
  );
}

/** 标记引用 mark：思考块内 `@` 触发的结构化引用（任务/设定/自定义） */
export const MarkupRef = Mark.create({
  name: "markupRef",

  inclusive: false,

  excludes: "",

  addAttributes() {
    return {
      kind: { default: "custom" },
      id: { default: "" },
      label: { default: "" },
      status: { default: "todo" },
      entityPath: { default: "" },
      field: { default: "" },
      value: { default: "" },
      tag: { default: "" },
      body: { default: "" },
    };
  },

  parseHTML() {
    return [{ tag: "span[data-markup-ref]" }];
  },

  renderHTML({ mark, HTMLAttributes }) {
    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-markup-ref": mark.attrs.kind,
        class: `markup-ref markup-${mark.attrs.kind}`,
      }),
      0,
    ];
  },
});
