import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  Sparkles,
  X,
  Check,
  RotateCcw,
  Download,
  Brain,
  ListChecks,
  BookMarked,
  Tag,
} from "lucide-react";
import { ThinkingBlock, MarkupRef } from "../editor/ThinkingBlock";
import { ModeSwitch, ModeChangeInfo } from "../editor/ModeSwitch";
import {
  editorToBlocks,
  blocksToDoc,
  BlockIdentity,
  buildTrainingExamples,
  serializeExamples,
  formatFilename,
  downloadText,
  ExportFormat,
  MarkupRef as MarkupRefType,
} from "../editor/blocks";
import type { ContentBlock } from "../types";
import { logger } from "../logger";

interface EditorProps {
  onTextChange: (text: string) => void;
  onIdle: () => void;
  onInsertText?: (text: string) => void;
  /** 领域事件定位：用于模式切换信号发射（浏览器模式自动降级） */
  projectId?: string | undefined;
  chapterId?: string | undefined;
  /** 切换章节时注入已保存正文；编辑器按 chapterId 重建 */
  initialText?: string | undefined;
  initialBlocks?: ContentBlock[] | undefined;
  onBlocksChange?: ((blocks: ContentBlock[]) => void) | undefined;
}

interface MentionMenuState {
  top: number;
  left: number;
  anchorFrom: number; // '@' 位置
}

const MENTION_ITEMS: Array<{
  kind: MarkupRefType["type"];
  icon: React.ReactNode;
  label: string;
  desc: string;
  attrs: Record<string, string>;
  text: string;
}> = [
  {
    kind: "task",
    icon: <ListChecks size={13} />,
    label: "任务",
    desc: "关联创作任务",
    attrs: { kind: "task", id: "", label: "新任务", status: "todo" },
    text: "[任务: 新任务]",
  },
  {
    kind: "setting",
    icon: <BookMarked size={13} />,
    label: "设定",
    desc: "引用自定义设定",
    attrs: {
      kind: "setting",
      entityPath: "自定义设定",
      field: "条目",
      value: "",
    },
    text: "[设定: 自定义设定.条目]",
  },
  {
    kind: "custom",
    icon: <Tag size={13} />,
    label: "标记",
    desc: "自定义标记",
    attrs: { kind: "custom", tag: "伏笔", body: "" },
    text: "[@伏笔: ]",
  },
];

export function Editor({
  onTextChange,
  onIdle,
  onInsertText,
  projectId,
  chapterId,
  initialText = "",
  initialBlocks,
  onBlocksChange,
}: EditorProps) {
  const [wordCount, setWordCount] = useState(0);
  const [thinkingCount, setThinkingCount] = useState(0);
  const [isTyping, setIsTyping] = useState(false);
  const [mention, setMention] = useState<MentionMenuState | null>(null);
  /** 类型标记渐隐动画开关：true 时编辑器内正文/思考/标签按类型着色并逐渐消失 */
  const [flashing, setFlashing] = useState(false);
  const idleTimer = useRef<ReturnType<typeof setTimeout>>();
  const flashTimer = useRef<ReturnType<typeof setTimeout>>();
  const mounted = useRef(true);

  /** 触发一次全编辑器类型着色渐隐（重放：先移除 class，下一帧再加） */
  const triggerFlash = useCallback(() => {
    if (!mounted.current) return;
    setFlashing(false);
    requestAnimationFrame(() => {
      if (mounted.current) setFlashing(true);
    });
    if (flashTimer.current) clearTimeout(flashTimer.current);
    flashTimer.current = setTimeout(() => {
      if (mounted.current) setFlashing(false);
    }, 1600);
  }, []);

  /** 模式切换信号量：前端事件 + 后端领域事件（→ 工作流规则 → 任务序列） */
  const emitModeChange = useCallback(
    (info: ModeChangeInfo) => {
      // 通道 1：前端事件，插件可在 window 上监听 novel:mode-changed
      window.dispatchEvent(
        new CustomEvent("novel:mode-changed", {
          detail: { ...info, projectId, chapterId },
        }),
      );
      // 通道 2：后端领域事件 block.mode.changed，驱动工作流任务序列
      if (projectId && chapterId && (window as any).__TAURI_INTERNALS__) {
        invoke<{ recorded: boolean; queued: number }>("emit_block_mode_changed", {
          projectId,
          chapterId,
          mode: info.mode,
          previousMode: info.previousMode,
          blockId: null,
          position: info.position,
        })
          .then((r) =>
            logger.info("模式切换事件已送达队列引擎", {
              recorded: r.recorded,
              queued: r.queued,
            }),
          )
          .catch((e) => logger.warn("模式切换事件发送失败", { error: String(e) }));
      } else {
        logger.info("模式切换（浏览器预览，事件仅前端派发）", {
          mode: info.mode,
          previousMode: info.previousMode,
        });
      }
    },
    [projectId, chapterId],
  );
  // 保持 useEditor 首次闭包拿到最新回调
  const emitModeChangeRef = useRef(emitModeChange);
  emitModeChangeRef.current = emitModeChange;

  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        heading: { levels: [1, 2, 3] },
      }),
      Placeholder.configure({
        placeholder: "开始你的创作...\n新行行首按 Tab 切换思考/正文",
      }),
      ThinkingBlock,
      MarkupRef,
      BlockIdentity,
      ModeSwitch.configure({
        onModeChanged: (info) => emitModeChangeRef.current(info),
      }),
    ],
    content:
      initialBlocks && initialBlocks.length > 0 ? blocksToDoc(initialBlocks) : initialText,
    editorProps: {
      attributes: {
        class: "novel-editor",
        spellcheck: "false",
      },
    },
    onTransaction: ({ editor, transaction }) => {
      // 换新行 / 切换模式（doc 结构变化且光标落在空块行首）→ 类型着色渐隐
      if (!transaction.docChanged) return;
      const { $from } = editor.state.selection;
      const parentType = $from.parent.type.name;
      if (parentType !== "paragraph" && parentType !== "thinkingBlock") return;
      if ($from.parent.textContent === "" && $from.parentOffset === 0) {
        triggerFlash();
      }
    },
    onUpdate: ({ editor }) => {
      const blocks = editorToBlocks(editor);
      const bodyText = blocks
        .filter((b) => b.kind === "body")
        .map((b) => b.text)
        .join("");
      const thinkBlocks = blocks.filter((b) => b.kind === "thinking");
      setWordCount(bodyText.length);
      setThinkingCount(thinkBlocks.length);
      setIsTyping(true);
      onTextChange(bodyText);
      onBlocksChange?.(blocks);

      if (idleTimer.current) clearTimeout(idleTimer.current);
      idleTimer.current = setTimeout(() => {
        setIsTyping(false);
        onIdle();
      }, 1800);
    },
    onSelectionUpdate: ({ editor }) => {
      maybeOpenMention(editor);
    },
    onBlur: () => setMention(null),
  }, [chapterId]);

  /** 检测光标前是否输入 `@`（仅思考块内触发补全） */
  const maybeOpenMention = useCallback(
    (ed: NonNullable<ReturnType<typeof useEditor>>) => {
      if (!ed) return;
      const { state } = ed;
      const { $from } = state.selection;
      if ($from.parent.type.name !== "thinkingBlock") {
        setMention(null);
        return;
      }
      const from = state.selection.from;
      if (from < 1) return;
      const prev = state.doc.textBetween(Math.max(0, from - 1), from);
      if (prev !== "@") {
        setMention(null);
        return;
      }
      const coords = ed.view.coordsAtPos(from);
      setMention({
        top: coords.bottom + 6,
        left: coords.left,
        anchorFrom: from - 1,
      });
    },
    [],
  );

  const insertMention = useCallback(
    (item: (typeof MENTION_ITEMS)[number]) => {
      if (!editor || !mention) return;
      editor
        .chain()
        .focus()
        .deleteRange({ from: mention.anchorFrom, to: mention.anchorFrom + 1 })
        .insertContent([
          {
            type: "text",
            text: item.text,
            marks: [{ type: "markupRef", attrs: item.attrs }],
          },
        ])
        .run();
      setMention(null);
    },
    [editor, mention],
  );

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      if (idleTimer.current) clearTimeout(idleTimer.current);
      if (flashTimer.current) clearTimeout(flashTimer.current);
    };
  }, []);

  useEffect(() => {
    if (onInsertText && editor) {
      (window as any).__editorInsert = (text: string) => {
        editor.chain().focus().insertContent(text).run();
      };
    }
  }, [editor, onInsertText]);

  const handleExport = useCallback(
    (format: ExportFormat) => {
      if (!editor) return;
      const blocks = editorToBlocks(editor);
      const examples = buildTrainingExamples(blocks, true);
      if (examples.length === 0) return;
      const output = serializeExamples(examples, format);
      downloadText(formatFilename(format), output);
    },
    [editor],
  );

  return (
    <div className={`editor-wrapper ${flashing ? "mode-flash" : ""}`}>
      <div className="editor-toolbar">
        <div className="editor-stats">
          <span className="stat">{wordCount} 字</span>
          {thinkingCount > 0 && (
            <span className="stat think">
              <Brain size={12} />
              {thinkingCount} 段思考
            </span>
          )}
          <span className={`stat status ${isTyping ? "typing" : ""}`}>
            {isTyping ? "输入中..." : "已停笔"}
          </span>
        </div>
        <div className="editor-actions">
          <div className="export-group">
            <button className="tool-btn" title="导出为 AI 训练数据" disabled={wordCount === 0}>
              <Download size={14} />
              导出
            </button>
            <div className="export-menu">
              <button onClick={() => handleExport("jsonl")}>JSONL · thinking/content 对</button>
              <button onClick={() => handleExport("sharegpt")}>ShareGPT · 对话格式</button>
              <button onClick={() => handleExport("r1")}>R1 风格 · &lt;think&gt; 标签</button>
            </div>
          </div>
          <button className="tool-btn" title="AI 续写">
            <Sparkles size={14} />
          </button>
        </div>
      </div>
      <div className="editor-body">
        <EditorContent editor={editor} />
        {mention && (
          <div
            className="mention-menu"
            style={{ top: mention.top, left: mention.left }}
            onMouseDown={(e) => e.preventDefault()}
          >
            {MENTION_ITEMS.map((item) => (
              <button key={item.kind} className="mention-item" onClick={() => insertMention(item)}>
                <span className="mention-icon">{item.icon}</span>
                <span className="mention-meta">
                  <span className="mention-label">{item.label}</span>
                  <span className="mention-desc">{item.desc}</span>
                </span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// AI 生成内容预览组件 - 内联显示在编辑器中
export function AIPreview({
  text,
  onAccept,
  onReject,
  onRevise,
}: {
  text: string;
  onAccept: () => void;
  onReject: () => void;
  onRevise: () => void;
}) {
  if (!text) return null;

  return (
    <div className="ai-preview-card">
      <div className="ai-preview-header">
        <div className="ai-badge">
          <Sparkles size={12} />
          <span>AI 续写</span>
        </div>
        <div className="ai-actions">
          <button className="ai-btn accept" onClick={onAccept} title="接受">
            <Check size={14} />
          </button>
          <button className="ai-btn revise" onClick={onRevise} title="重新生成">
            <RotateCcw size={14} />
          </button>
          <button className="ai-btn reject" onClick={onReject} title="拒绝">
            <X size={14} />
          </button>
        </div>
      </div>
      <div className="ai-preview-content">
        <p>{text}</p>
      </div>
      <div className="ai-preview-footer">
        <span className="hint">按 Tab 接受 · Esc 拒绝</span>
      </div>
    </div>
  );
}
