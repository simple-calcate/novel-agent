import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import { useEffect, useRef, useState } from "react";
import { Sparkles, X, Check, RotateCcw } from "lucide-react";

interface EditorProps {
  onTextChange: (text: string) => void;
  onIdle: () => void;
  onInsertText?: (text: string) => void;
}

export function Editor({ onTextChange, onIdle, onInsertText }: EditorProps) {
  const [wordCount, setWordCount] = useState(0);
  const [isTyping, setIsTyping] = useState(false);
  const idleTimer = useRef<ReturnType<typeof setTimeout>>();

  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        heading: { levels: [1, 2, 3] },
      }),
      Placeholder.configure({
        placeholder: "开始你的创作...",
      }),
    ],
    content: "",
    editorProps: {
      attributes: {
        class: "novel-editor",
        spellcheck: "false",
      },
    },
    onUpdate: ({ editor }) => {
      const text = editor.getText();
      setWordCount(text.length);
      setIsTyping(true);
      onTextChange(text);

      if (idleTimer.current) clearTimeout(idleTimer.current);
      idleTimer.current = setTimeout(() => {
        setIsTyping(false);
        onIdle();
      }, 1800);
    },
  });

  useEffect(() => {
    return () => {
      if (idleTimer.current) clearTimeout(idleTimer.current);
    };
  }, []);

  useEffect(() => {
    if (onInsertText && editor) {
      (window as any).__editorInsert = (text: string) => {
        editor.chain().focus().insertContent(text).run();
      };
    }
  }, [editor, onInsertText]);

  return (
    <div className="editor-wrapper">
      <div className="editor-toolbar">
        <div className="editor-stats">
          <span className="stat">{wordCount} 字</span>
          <span className={`stat status ${isTyping ? "typing" : ""}`}>
            {isTyping ? "输入中..." : "已停笔"}
          </span>
        </div>
        <div className="editor-actions">
          <button className="tool-btn" title="AI 续写">
            <Sparkles size={14} />
          </button>
        </div>
      </div>
      <EditorContent editor={editor} />
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
