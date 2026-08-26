import { useState } from "react";
import {
  BookOpen,
  Brain,
  CheckCircle2,
  CircleDot,
  FileText,
  Layers,
  ListChecks,
  MessageSquare,
  PenLine,
  Plus,
  Settings,
  Sparkles,
  Terminal,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { libraryApi } from "./api";
import { Editor, AIPreview } from "./components/Editor";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { ContextRail } from "./components/ContextRail";
import { WorkflowPanel } from "./components/WorkflowPanel";
import { SettingsModal } from "./components/SettingsModal";
import { LogPanel } from "./components/LogPanel";
import { CreateDialog } from "./components/CreateDialog";
import { ConfirmDialog, TreeItemActions } from "./components/LibraryActions";
import { logger } from "./logger";
import { useLibrary } from "./hooks/useLibrary";
import { useQueue } from "./hooks/useQueue";
import { useEditorSession } from "./hooks/useEditorSession";

export function App() {
  const [sidebarTab, setSidebarTab] = useState<"context" | "workflow" | "agent">("context");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [logPanelOpen, setLogPanelOpen] = useState(false);
  const library = useLibrary();
  const {
    projects,
    project,
    books,
    chapters,
    activeChapter,
    setActiveChapter,
    setActiveBookId,
    libraryError,
    prompt,
    setPrompt,
    pendingDelete,
    setPendingDelete,
    applyLibrary,
    handlePrompt,
    handleDelete,
    mutateBook,
    mutateChapter,
  } = library;
  const { jobs, queueReady, enqueue } = useQueue(project);
  const session = useEditorSession({
    project,
    chapters,
    activeChapter,
    setActiveBookId,
  });
  const {
    chapterText,
    chapterBlocks,
    chapterReady,
    hints,
    aiPreview,
    revision,
    modelConfig,
    setModelConfig,
    draftText,
    draftBlocks,
    persistChapter,
    refreshHints,
    handleGenerate,
    handleAccept,
    handleReject,
  } = session;

  const activeChapterRecord = chapters.find((chapter) => chapter.id === activeChapter);
  const promptCopy = prompt
    ? prompt.mode === "rename"
      ? {
          title:
            prompt.target === "project" ? "重命名作品" : prompt.target === "book" ? "重命名书" : "重命名章节",
          label: "名称",
          placeholder: prompt.title ?? "",
          confirm: "保存",
        }
      : prompt.target === "project"
        ? { title: "新作品", label: "作品名称", placeholder: "例如：夜航星图", confirm: "创建" }
        : prompt.target === "book"
          ? { title: "新书", label: "书名 / 卷名", placeholder: "例如：卷一 · 雾与海", confirm: "创建" }
          : { title: "新章节", label: "章节标题", placeholder: "例如：第一章 雾港来客", confirm: "创建" }
    : { title: "", label: "", placeholder: "", confirm: "创建" };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <PenLine size={20} />
          </div>
          <div>
            <div className="brand-name">墨枢</div>
            <div className="brand-sub">Novel Agent</div>
          </div>
        </div>

        <div className="project-card">
          <div className="project-label">当前作品</div>
          {projects.length > 0 ? (
            <select
              className="project-select"
              value={project?.id ?? ""}
              onChange={(event) => {
                const id = event.target.value;
                void libraryApi.setActiveProject(id).then(applyLibrary);
              }}
            >
              {projects.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.title}
                </option>
              ))}
            </select>
          ) : (
            <div className="project-title">尚未创建作品</div>
          )}
          <div className="project-meta">
            <span>本地优先</span>
            <span>·</span>
            <span>{books.length} 本书</span>
          </div>
          <div className="project-actions">
            <button className="text-button" onClick={() => setPrompt({ mode: "create", target: "project" })}>
              新作品
            </button>
            <button className="text-button" onClick={() => setPrompt({ mode: "create", target: "book" })}>
              新书
            </button>
            {project && (
              <>
                <button
                  className="text-button"
                  onClick={() =>
                    setPrompt({ mode: "rename", target: "project", id: project.id, title: project.title })
                  }
                >
                  重命名
                </button>
                <button
                  className="text-button"
                  onClick={() => setPendingDelete({ target: "project", id: project.id, title: project.title })}
                >
                  删除
                </button>
              </>
            )}
          </div>
        </div>

        <nav className="tree">
          {libraryError && <div className="tree-empty">{libraryError}</div>}
          {books.length === 0 && (
            <div className="tree-empty">还没有书。点上方「新书」创建第一本。</div>
          )}
          {books.map((book, bookIndex) => {
            const bookChapters = chapters.filter((chapter) => chapter.bookId === book.id);
            return (
              <div key={book.id} className="tree-book">
                <div className="tree-section">
                  <Layers size={14} />
                  <span>{book.title}</span>
                  <TreeItemActions
                    disableUp={bookIndex === 0}
                    disableDown={bookIndex === books.length - 1}
                    onRename={() =>
                      setPrompt({ mode: "rename", target: "book", id: book.id, title: book.title })
                    }
                    onDelete={() => setPendingDelete({ target: "book", id: book.id, title: book.title })}
                    onMoveUp={() => void mutateBook(book.id, -1)}
                    onMoveDown={() => void mutateBook(book.id, 1)}
                  />
                </div>
                {bookChapters.map((chapter, chapterIndex) => (
                  <div
                    key={chapter.id}
                    className={`tree-item ${activeChapter === chapter.id ? "active" : ""}`}
                    onClick={() => {
                      void persistChapter();
                      setActiveChapter(chapter.id);
                      setActiveBookId(book.id);
                    }}
                  >
                    <FileText size={14} />
                    <span>{chapter.title}</span>
                    <TreeItemActions
                      disableUp={chapterIndex === 0}
                      disableDown={chapterIndex === bookChapters.length - 1}
                      onRename={() =>
                        setPrompt({
                          mode: "rename",
                          target: "chapter",
                          id: chapter.id,
                          title: chapter.title,
                        })
                      }
                      onDelete={() =>
                        setPendingDelete({ target: "chapter", id: chapter.id, title: chapter.title })
                      }
                      onMoveUp={() => void mutateChapter(chapter.id, -1)}
                      onMoveDown={() => void mutateChapter(chapter.id, 1)}
                    />
                  </div>
                ))}
              </div>
            );
          })}
          <button
            className="tree-item add"
            onClick={() => setPrompt({ mode: "create", target: "chapter" })}
            disabled={!project || books.length === 0}
          >
            <Plus size={14} />
            <span>新章节</span>
          </button>
        </nav>

        <div className="sidebar-footer">
          <button className="icon-button" title="设置" onClick={() => setSettingsOpen(true)}>
            <Settings size={16} />
          </button>
          <button className="icon-button" title="插件">
            <Layers size={16} />
          </button>
          <button className="icon-button" title="任务队列">
            <ListChecks size={16} />
          </button>
          <button
            className={`icon-button ${logPanelOpen ? "active" : ""}`}
            title="日志"
            onClick={() => setLogPanelOpen(!logPanelOpen)}
          >
            <Terminal size={16} />
          </button>
        </div>
      </aside>

      <main className="workspace">
        <header className="topbar">
          <div className="chapter-title">
            <BookOpen size={16} />
            <span>{activeChapterRecord?.title ?? "未选择章节"}</span>
            <span className="revision-badge">R{revision}</span>
          </div>
          <div className="topbar-actions">
            <button
              className="action-button ghost"
              disabled={!project}
              onClick={() => enqueue("continuity.check")}
            >
              <CheckCircle2 size={14} />
              检查
            </button>
            <button className="action-button primary" onClick={handleGenerate}>
              <Sparkles size={14} />
              续写
            </button>
          </div>
        </header>

        <div className="editor-area">
          {!activeChapter && (
            <div className="workspace-empty">
              <p>从左侧创建作品、书和章节，即可开始写作。</p>
              <div className="workspace-empty-actions">
                <button className="btn primary" onClick={() => setPrompt({ mode: "create", target: "book" })}>
                  创建书籍
                </button>
                <button className="btn" onClick={() => setPrompt({ mode: "create", target: "project" })}>
                  仅创建作品
                </button>
              </div>
            </div>
          )}
          {activeChapter && chapterReady && (
            <ErrorBoundary label="编辑器">
              <Editor
                key={activeChapter}
                initialText={chapterText}
                initialBlocks={chapterBlocks}
                projectId={project?.id}
                chapterId={activeChapter}
                chapterTitle={activeChapterRecord?.title}
                onTextChange={(text) => {
                  draftText.current = text;
                  refreshHints(text.slice(-300));
                }}
                onBlocksChange={(blocks) => {
                  draftBlocks.current = blocks;
                }}
                onIdle={() => {
                  void persistChapter();
                  enqueue("index.rebuild");
                }}
              />
            </ErrorBoundary>
          )}
          <ContextRail hints={hints} />

          {aiPreview && (
            <AIPreview
              text={aiPreview}
              onAccept={handleAccept}
              onReject={handleReject}
              onRevise={handleGenerate}
            />
          )}
        </div>
      </main>

      <aside className="right-panel">
        <div className="panel-tabs">
          <button
            className={sidebarTab === "context" ? "active" : ""}
            onClick={() => setSidebarTab("context")}
          >
            <Brain size={14} />
            上下文
          </button>
          <button
            className={sidebarTab === "workflow" ? "active" : ""}
            onClick={() => setSidebarTab("workflow")}
          >
            <ListChecks size={14} />
            工作流
          </button>
          <button
            className={sidebarTab === "agent" ? "active" : ""}
            onClick={() => setSidebarTab("agent")}
          >
            <MessageSquare size={14} />
            Agent
          </button>
        </div>

        {sidebarTab === "context" && (
          <div className="panel-content">
            <h3>这一段怎么写</h3>
            <div className="context-card">
              <div className="context-card-title">
                <CircleDot size={12} />
                1. 先写思考
              </div>
              <p>
                空行按 Tab。淡紫色块是给自己看的：这段要干什么、现在还不能揭什么。读者看不到。
              </p>
            </div>
            <div className="context-card">
              <div className="context-card-title">
                <CircleDot size={12} />
                2. 再写正文
              </div>
              <p>
                再按 Tab，写读者看到的小说。角色心里想什么写这里，不要写进思考。
              </p>
            </div>
            <div className="context-card">
              <div className="context-card-title">
                <CircleDot size={12} />
                3. 一小段一小段
              </div>
              <p>
                想一下 → 写几句 → 再想一下。章首先写完大纲再一口气写全章，以后 AI 学不会你在光标处怎么续。
              </p>
            </div>
          </div>
        )}

        {sidebarTab === "workflow" && (
          <WorkflowPanel
            jobs={jobs}
            queueReady={queueReady}
            onRun={(operation, label) => {
              logger.info("手动运行工作流", { operation, label });
              enqueue(operation);
            }}
          />
        )}

        {sidebarTab === "agent" && (
          <div className="panel-content">
            <h3>Agent 会话</h3>
            <div className="agent-message">
              <strong>系统</strong>
              <p>
                {project
                  ? `当前作品「${project.title}」，上下文固定到 Revision ${revision}。`
                  : "创建作品后即可把 Agent 会话钉在该书的修订历史上。"}
              </p>
            </div>
          </div>
        )}
      </aside>

      <SettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        initialConfig={modelConfig}
        onSave={async (config) => {
          logger.info("保存模型配置", { provider: config.provider, model: config.model });
          setModelConfig(config);
          try {
            await invoke("save_model_config", { config });
            logger.info("配置已同步到后端");
          } catch (e) {
            logger.error("保存失败", { error: String(e) });
          }
        }}
      />

      <LogPanel open={logPanelOpen} onClose={() => setLogPanelOpen(false)} />

      <CreateDialog
        open={prompt !== null}
        title={promptCopy.title}
        label={promptCopy.label}
        placeholder={promptCopy.placeholder}
        confirmLabel={promptCopy.confirm}
        initialValue={prompt?.mode === "rename" ? (prompt.title ?? "") : ""}
        onClose={() => setPrompt(null)}
        onSubmit={handlePrompt}
      />
      <ConfirmDialog
        open={pendingDelete !== null}
        title={
          pendingDelete?.target === "project"
            ? "删除作品"
            : pendingDelete?.target === "book"
              ? "删除书"
              : "删除章节"
        }
        body={`确定删除「${pendingDelete?.title ?? ""}」？此操作不可撤销。`}
        onClose={() => setPendingDelete(null)}
        onConfirm={handleDelete}
      />
    </div>
  );
}
