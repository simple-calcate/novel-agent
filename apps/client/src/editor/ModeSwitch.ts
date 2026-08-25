import { Extension } from "@tiptap/core";
import type { Editor } from "@tiptap/core";

export type ModeKind = "body" | "thinking";

export interface ModeChangeInfo {
  mode: ModeKind;
  previousMode: ModeKind;
  /** 切换前光标所在文档位置（块起始 pos） */
  position: number;
}

/**
 * Tab 上下文：定义"某个状态下 Tab 键的行为"。
 * Tab 不是全局快捷键——只有 canActivate 判定当前状态生效时才会消费按键，
 * 其余状态交还默认行为。未来其他状态（AI 预览、批注等）可注册自己的上下文。
 */
export interface TabContext {
  id: string;
  canActivate: (editor: Editor) => boolean;
  run: (editor: Editor) => boolean;
}

export interface ModeSwitchOptions {
  /** 模式切换完成后的回调（视觉标记 + 信号发射的挂载点） */
  onModeChanged?: (info: ModeChangeInfo) => void;
  /** 扩展点：额外的 Tab 上下文，按序尝试，第一个 canActivate 生效 */
  tabContexts?: TabContext[];
}

/**
 * 是否处于"新行行首"状态：光标在空块起始处，且块是正文/思考。
 * 这是 Tab 切换模式生效的唯一状态。
 */
export function isFreshLineState(opts: {
  parentType: string;
  parentOffset: number;
  textLength: number;
}): boolean {
  return (
    opts.parentOffset === 0 &&
    opts.textLength === 0 &&
    (opts.parentType === "paragraph" || opts.parentType === "thinkingBlock")
  );
}

/** 当前块类型切换后的目标模式；非正文/思考返回 null（不可切换） */
export function nextMode(parentType: string): ModeKind | null {
  if (parentType === "paragraph") return "thinking";
  if (parentType === "thinkingBlock") return "body";
  return null;
}

/** 内置上下文：新行行首 Tab 切换思考/正文 */
function freshLineContext(onModeChanged?: (info: ModeChangeInfo) => void): TabContext {
  return {
    id: "fresh-line-mode-toggle",
    canActivate: (editor) => {
      const { $from } = editor.state.selection;
      return isFreshLineState({
        parentType: $from.parent.type.name,
        parentOffset: $from.parentOffset,
        textLength: $from.parent.textContent.length,
      });
    },
    run: (editor) => {
      const { $from } = editor.state.selection;
      const schema = editor.state.schema;
      const parentType = $from.parent.type.name;
      const mode = nextMode(parentType);
      if (!mode) return false;

      const previousMode: ModeKind = parentType === "thinkingBlock" ? "thinking" : "body";
      const position = $from.pos;

      if (mode === "thinking") {
        editor.chain().setNode(schema.nodes.thinkingBlock).run();
      } else {
        editor.chain().setParagraph().run();
      }
      onModeChanged?.({ mode, previousMode, position });
      return true;
    },
  };
}

/**
 * 模式切换状态机。
 * - Tab 仅在"新行行首"状态切换思考/正文，其他状态不拦截（默认行为）。
 * - tabContexts 提供扩展点：同一键在不同状态下可有不同效果。
 */
export const ModeSwitch = Extension.create<ModeSwitchOptions>({
  name: "modeSwitch",

  addOptions(): ModeSwitchOptions {
    return {
      tabContexts: [],
    };
  },

  addKeyboardShortcuts() {
    return {
      Tab: ({ editor }) => {
        const contexts = [
          freshLineContext(this.options.onModeChanged),
          ...(this.options.tabContexts ?? []),
        ];
        for (const context of contexts) {
          if (context.canActivate(editor)) {
            return context.run(editor);
          }
        }
        return false;
      },
    };
  },
});
