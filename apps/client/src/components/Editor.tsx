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
  User,
  Flame,
  MapPin,
  Package,
  Users,
  Scale,
} from "lucide-react";
import { ThinkingBlock, MarkupRef } from "../editor/ThinkingBlock";
import { ModeSwitch, ModeChangeInfo } from "../editor/ModeSwitch";
import {
  editorToBlocks,
  blocksToDoc,
  BlockIdentity,
  buildTrainingExamples,
  serializeExamples,
  filterExamples,
  formatFilename,
  downloadText,
  ExportFormat,
} from "../editor/blocks";
import { WritingGuide } from "./WritingGuide";
import { FloatingMenu } from "./FloatingMenu";
import {
  authorExportSummary,
  countMissingThinking,
  guideCopy,
  writerModeFromParent,
  type WriterMode,
} from "../editor/guide";
import { StoryCatalogContext } from "../editor/StoryCatalog";
import { filterTagKindLabels } from "../structure/tagCandidates";
import type { ContentBlock, StoryEntry } from "../types";
import { logger } from "../logger";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    __editorInsert?: (text: string) => void;
  }
}

interface EditorProps {
  onTextChange: (text: string) => void;
  /** 当前光标所在段落，以及上一段（用于人物短暂停留）。 */
  onNearbyChange?: (nearby: { current: string; previous: string }) => void;
  onIdle: () => void;
  onInsertText?: (text: string) => void;
  /** 领域事件定位：用于模式切换信号发射（浏览器模式自动降级） */
  projectId?: string | undefined;
  chapterId?: string | undefined;
  chapterTitle?: string | undefined;
  /** 切换章节时注入已保存正文；编辑器按 chapterId 重建 */
  initialText?: string | undefined;
  initialBlocks?: ContentBlock[] | undefined;
  onBlocksChange?: ((blocks: ContentBlock[]) => void) | undefined;
  storyEntries?: StoryEntry[];
}

interface MentionMenuState {
  top: number;
  left: number;
  bottom: number;
  anchorFrom: number;
  query: string;
}

const MENTION_ITEMS: Array<{
  kind: "tag";
  icon: React.ReactNode;
  label: string;
  desc: string;
  attrs: Record<string, string>;
}> = [
  {
    kind: "tag",
    icon: <User size={13} />,
    label: "人物",
    desc: "点名角色。写作标签，以后再拆成工具",
    attrs: { kind: "tag", tagKind: "人物", id: "", label: "", note: "" },
  },
  {
    kind: "tag",
    icon: <Flame size={13} />,
    label: "伏笔",
    desc: "点一条伏笔。先当标签，不必对上正史库",
    attrs: { kind: "tag", tagKind: "伏笔", id: "", label: "", note: "" },
  },
  {
    kind: "tag",
    icon: <MapPin size={13} />,
    label: "地点",
    desc: "点一个地点",
    attrs: { kind: "tag", tagKind: "地点", id: "", label: "", note: "" },
  },
  {
    kind: "tag",
    icon: <Package size={13} />,
    label: "道具",
    desc: "点一件物件",
    attrs: { kind: "tag", tagKind: "道具", id: "", label: "", note: "" },
  },
  {
    kind: "tag",
    icon: <Users size={13} />,
    label: "势力",
    desc: "点一个组织或势力",
    attrs: { kind: "tag", tagKind: "势力", id: "", label: "", note: "" },
  },
  {
    kind: "tag",
    icon: <Scale size={13} />,
    label: "规则",
    desc: "点一条世界规则",
    attrs: { kind: "tag", tagKind: "规则", id: "", label: "", note: "" },
  },
];

export function Editor({
  onTextChange,
  onNearbyChange,
  onIdle,
  onInsertText,
  projectId,
  chapterId,
  chapterTitle,
  initialText = "",
  initialBlocks,
  onBlocksChange,
  storyEntries = [],
}: EditorProps) {
  const [wordCount, setWordCount] = useState(0);
  const [thinkingCount, setThinkingCount] = useState(0);
  const [writerMode, setWriterMode] = useState<WriterMode>("body");
  const [missingThinking, setMissingThinking] = useState(0);
  const [exportNotice, setExportNotice] = useState<string | null>(null);
  const [isTyping, setIsTyping] = useState(false);
  const [mention, setMention] = useState<MentionMenuState | null>(null);
  const [mentionIndex, setMentionIndex] = useState(0);
  const [nearbyText, setNearbyText] = useState("");
  /** 类型标记渐隐动画开关：true 时编辑器内正文/思考/标签按类型着色并逐渐消失 */
  const [flashing, setFlashing] = useState(false);
  const idleTimer = useRef<ReturnType<typeof setTimeout>>();
  const flashTimer = useRef<ReturnType<typeof setTimeout>>();
  /** 重计算防抖：按键路径只做轻量状态，块转换/训练样例/浮带匹配延迟到停顿后 */
  const heavyTimer = useRef<ReturnType<typeof setTimeout>>();
  const nearbyTimer = useRef<ReturnType<typeof setTimeout>>();
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
      if (projectId && chapterId && window.__TAURI_INTERNALS__) {
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
  const onNearbyChangeRef = useRef(onNearbyChange);
  onNearbyChangeRef.current = onNearbyChange;
  const lastNearby = useRef<string | null>(null);

  // onUpdate/onSelectionUpdate 闭包只在 chapterId 变化时重建，用 ref 保持最新值
  const chapterTitleRef = useRef(chapterTitle);
  chapterTitleRef.current = chapterTitle;
  const onTextChangeRef = useRef(onTextChange);
  onTextChangeRef.current = onTextChange;
  const onBlocksChangeRef = useRef(onBlocksChange);
  onBlocksChangeRef.current = onBlocksChange;
  const mentionRef = useRef(mention);
  mentionRef.current = mention;
  const mentionIndexRef = useRef(mentionIndex);
  mentionIndexRef.current = mentionIndex;
  const mentionItemsRef = useRef<typeof MENTION_ITEMS>([]);
  const insertMentionRef = useRef<(item: (typeof MENTION_ITEMS)[number]) => void>(() => {});

  /** 挂起的重计算：卸载时同步冲刷，避免切章节丢失最后一段草稿 */
  const pendingHeavy = useRef<(() => void) | null>(null);
  const flushHeavy = () => {
    if (heavyTimer.current) {
      clearTimeout(heavyTimer.current);
      heavyTimer.current = undefined;
    }
    const pending = pendingHeavy.current;
    pendingHeavy.current = null;
    pending?.();
  };

  const reportNearby = useCallback((ed: NonNullable<ReturnType<typeof useEditor>>) => {
    if (!ed) return;
    const nearby = paragraphWindow(ed);
    const key = `${nearby.current}\n${nearby.previous}`;
    if (lastNearby.current === key) return;
    lastNearby.current = key;
    setNearbyText(key);
    onNearbyChangeRef.current?.(nearby);
  }, []);

  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        heading: { levels: [1, 2, 3] },
      }),
      Placeholder.configure({
        placeholder: ({ node }) =>
          node.type.name === "thinkingBlock"
            ? ""
            : "写给读者的正文。空行按 Tab 切到思考",
        includeChildren: true,
        showOnlyCurrent: false,
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
      handleKeyDown: (_view, event) => {
        const open = mentionRef.current;
        const items = mentionItemsRef.current;
        if (!open || items.length === 0) return false;
        if (event.key === "Escape") {
          setMention(null);
          return true;
        }
        if (event.key === "ArrowDown") {
          setMentionIndex((index) => (index + 1) % items.length);
          return true;
        }
        if (event.key === "ArrowUp") {
          setMentionIndex((index) => (index - 1 + items.length) % items.length);
          return true;
        }
        if (event.key === "Enter" || event.key === "Tab") {
          const item = items[mentionIndexRef.current];
          if (item) {
            insertMentionRef.current(item);
            return true;
          }
        }
        return false;
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
      // 轻量路径：只更新光标模式与输入状态，保证击键 <16ms
      setWriterMode(writerModeFromParent(editor.state.selection.$from.parent.type.name));
      setIsTyping(true);
      maybeOpenMention(editor);

      // 重计算尾随防抖：块转换、训练样例、浮带匹配都是全文档遍历
      if (heavyTimer.current) clearTimeout(heavyTimer.current);
      const runHeavy = () => {
        pendingHeavy.current = null;
        if (!mounted.current) return;
        const ed = editor;
        const blocks = editorToBlocks(ed);
        const bodyText = blocks
          .filter((b) => b.kind === "body")
          .map((b) => b.text)
          .join("\n");
        const thinkBlocks = blocks.filter((b) => b.kind === "thinking");
        const examples = buildTrainingExamples(blocks, true, chapterTitleRef.current);
        setWordCount(bodyText.length);
        setThinkingCount(thinkBlocks.length);
        setMissingThinking(countMissingThinking(examples));
        onTextChangeRef.current(bodyText);
        onBlocksChangeRef.current?.(blocks);
        reportNearby(ed);
      };
      pendingHeavy.current = runHeavy;
      heavyTimer.current = setTimeout(runHeavy, 250);

      if (idleTimer.current) clearTimeout(idleTimer.current);
      idleTimer.current = setTimeout(() => {
        setIsTyping(false);
        onIdle();
      }, 1800);
    },
    onSelectionUpdate: ({ editor }) => {
      setWriterMode(writerModeFromParent(editor.state.selection.$from.parent.type.name));
      maybeOpenMention(editor);
      // 光标随击键高频移动，浮带匹配做全文档遍历，需要防抖
      if (nearbyTimer.current) clearTimeout(nearbyTimer.current);
      nearbyTimer.current = setTimeout(() => {
        nearbyTimer.current = undefined;
        if (mounted.current) reportNearby(editor);
      }, 150);
    },
    onCreate: ({ editor }) => {
      reportNearby(editor);
    },
    onBlur: () => setMention(null),
  }, [chapterId]);

  /** 思考块内 `@` 后的标签种类补全。 */
  const maybeOpenMention = useCallback(
    (ed: NonNullable<ReturnType<typeof useEditor>>) => {
      if (!ed) return;
      const found = mentionQueryAt(ed);
      if (!found) {
        setMention(null);
        return;
      }
      const labels = filterTagKindLabels(found.query);
      if (labels.length === 0) {
        setMention(null);
        return;
      }
      const coords = ed.view.coordsAtPos(ed.state.selection.from);
      const prev = mentionRef.current;
      if (prev?.anchorFrom !== found.anchorFrom || prev.query !== found.query) {
        setMentionIndex(0);
      }
      setMention({
        top: coords.top,
        left: coords.left,
        bottom: coords.bottom,
        anchorFrom: found.anchorFrom,
        query: found.query,
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
        .deleteRange({ from: mention.anchorFrom, to: editor.state.selection.from })
        .insertContent({
          type: "markupRef",
          attrs: item.attrs,
        })
        .run();
      setMention(null);
    },
    [editor, mention],
  );
  insertMentionRef.current = insertMention;

  useEffect(() => {
    if (!mention || !editor) return;
    const update = () => {
      try {
        const coords = editor.view.coordsAtPos(editor.state.selection.from);
        setMention((current) =>
          current
            ? { ...current, top: coords.top, left: coords.left, bottom: coords.bottom }
            : null,
        );
      } catch {
        setMention(null);
      }
    };
    const dom = editor.view.dom;
    dom.addEventListener("scroll", update);
    window.addEventListener("scroll", update, true);
    window.addEventListener("resize", update);
    return () => {
      dom.removeEventListener("scroll", update);
      window.removeEventListener("scroll", update, true);
      window.removeEventListener("resize", update);
    };
  }, [mention ? 1 : 0, editor]);

  useEffect(() => {
    mounted.current = true;
    return () => {
      // 先冲刷挂起的重计算（把草稿交给上层），再标记卸载
      flushHeavy();
      mounted.current = false;
      if (idleTimer.current) clearTimeout(idleTimer.current);
      if (flashTimer.current) clearTimeout(flashTimer.current);
      if (nearbyTimer.current) clearTimeout(nearbyTimer.current);
    };
  }, []);

  useEffect(() => {
    if (onInsertText && editor) {
      window.__editorInsert = (text: string) => {
        editor.chain().focus().insertContent(text).run();
      };
    }
  }, [editor, onInsertText]);

  const insertSlot = useCallback(
    (text: string) => {
      if (!editor) return;
      const type = editor.state.schema.nodes.thinkingBlock;
      const { $from } = editor.state.selection;
      if ($from.parent.type !== type) {
        if ($from.parent.textContent.length === 0) {
          editor.chain().focus().setNode(type).insertContent(text).run();
        }
        return;
      }
      if ($from.parent.textContent.length === 0) {
        editor.chain().focus().insertContent(text).run();
        return;
      }
      if ($from.parent.textContent.startsWith(text)) return;
      editor.chain().focus().splitBlock().insertContent(text).run();
    },
    [editor],
  );

  const handleExport = useCallback(
    (format: ExportFormat) => {
      if (!editor) return;
      const blocks = editorToBlocks(editor);
      const all = buildTrainingExamples(blocks, true, chapterTitle);
      const examples = filterExamples(all, "usable");
      const missing = countMissingThinking(all);
      setExportNotice(authorExportSummary(examples.length, missing));
      if (examples.length === 0) return;
      const output = serializeExamples(examples, format);
      downloadText(formatFilename(format), output);
    },
    [editor, chapterTitle],
  );

  const copy = guideCopy({ mode: writerMode, missingThinkingBeats: missingThinking });
  const mentionItems = mention
    ? MENTION_ITEMS.filter((item) => filterTagKindLabels(mention.query).includes(item.label))
    : [];
  mentionItemsRef.current = mentionItems;

  return (
    <StoryCatalogContext.Provider value={{ entries: storyEntries, nearby: nearbyText }}>
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
            <button className="tool-btn" title="导出本章（正文+思考）" disabled={wordCount === 0}>
              <Download size={14} />
              导出
            </button>
            <div className="export-menu">
              <button onClick={() => handleExport("jsonl")}>JSONL · 协议字段</button>
              <button onClick={() => handleExport("sharegpt")}>ShareGPT · 含系统短指令</button>
              <button onClick={() => handleExport("alpaca")}>Alpaca · instruction/input/output</button>
              <button onClick={() => handleExport("r1")}>R1 风格 · &lt;think&gt; 标签</button>
            </div>
          </div>
          <button className="tool-btn" title="AI 续写">
            <Sparkles size={14} />
          </button>
        </div>
      </div>
      <WritingGuide
        mode={writerMode}
        title={copy.title}
        body={copy.body}
        onInsertSlot={insertSlot}
      />
      {exportNotice && (
        <div className="export-notice" role="status">
          {exportNotice}
          <button type="button" className="export-notice-dismiss" onClick={() => setExportNotice(null)}>
            知道了
          </button>
        </div>
      )}
      <div className="editor-body">
        <EditorContent editor={editor} />
        {mention && mentionItems.length > 0 && (
          <FloatingMenu
            caret={{ top: mention.top, left: mention.left, bottom: mention.bottom }}
            itemCount={mentionItems.length}
            onMouseDown={(event) => event.preventDefault()}
          >
            {mentionItems.map((item, index) => (
              <button
                key={item.label}
                className={`mention-item${index === mentionIndex ? " active" : ""}`}
                onClick={() => insertMention(item)}
              >
                <span className="mention-icon">{item.icon}</span>
                <span className="mention-meta">
                  <span className="mention-label">{item.label}</span>
                  <span className="mention-desc">{item.desc}</span>
                </span>
              </button>
            ))}
          </FloatingMenu>
        )}
      </div>
    </div>
    </StoryCatalogContext.Provider>
  );
}

function mentionQueryAt(ed: {
  state: {
    selection: {
      from: number;
      $from: { parent: { type: { name: string } }; start: () => number };
    };
    doc: { textBetween: (from: number, to: number) => string };
  };
}): { anchorFrom: number; query: string } | null {
  const { $from, from } = ed.state.selection;
  if ($from.parent.type.name !== "thinkingBlock") return null;
  const start = $from.start();
  let query = "";
  for (let pos = from; pos > start; pos -= 1) {
    const ch = ed.state.doc.textBetween(pos - 1, pos);
    if (ch === "@") return { anchorFrom: pos - 1, query };
    if (!ch || /\s/.test(ch)) return null;
    query = `${ch}${query}`;
    if (query.length > 12) return null;
  }
  return null;
}

function paragraphWindow(editor: {
  state: {
    selection: { from: number };
    doc: {
      descendants: (
        fn: (node: { isTextblock: boolean; textContent: string; nodeSize: number }, pos: number) => boolean,
      ) => void;
    };
  };
}): { current: string; previous: string } {
  const from = editor.state.selection.from;
  const blocks: Array<{ text: string; pos: number; size: number }> = [];
  editor.state.doc.descendants((node, pos) => {
    if (node.isTextblock) {
      blocks.push({ text: node.textContent.trim(), pos, size: node.nodeSize });
    }
    return true;
  });
  let index = blocks.findIndex((block) => from >= block.pos && from <= block.pos + block.size);
  if (index < 0) index = blocks.length - 1;
  let current = blocks[index]?.text ?? "";
  if (!current) {
    for (let i = index - 1; i >= 0; i -= 1) {
      if (blocks[i]?.text) {
        current = blocks[i].text;
        index = i;
        break;
      }
    }
  }
  let previous = "";
  for (let i = index - 1; i >= 0; i -= 1) {
    if (blocks[i]?.text) {
      previous = blocks[i].text;
      break;
    }
  }
  return { current, previous };
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
