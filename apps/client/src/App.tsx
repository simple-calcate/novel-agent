import { useState } from "react";
import {
  Bookmark,
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
import { StructurePanel } from "./components/StructurePanel";
import { SettingsModal } from "./components/SettingsModal";
import { LogPanel } from "./components/LogPanel";
import { CreateDialog } from "./components/CreateDialog";
import { ConfirmDialog, TreeItemActions } from "./components/LibraryActions";
import { logger } from "./logger";
import { useLibrary } from "./hooks/useLibrary";
import { useQueue } from "./hooks/useQueue";
import { useEditorSession } from "./hooks/useEditorSession";
import { useStructure } from "./hooks/useStructure";

export function App() {
  const [sidebarTab, setSidebarTab] = useState<"context" | "structure" | "workflow" | "agent">(
    "structure",
  );
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [logPanelOpen, setLogPanelOpen] = useState(false);
  const library = useLibrary();
  const {
    projects,
    project,
    books,
    volumes,
    chapters,
    activeChapter,
    setActiveChapter,
    setActiveBookId,
    activeVolumeId,
    setActiveVolumeId,
    libraryError,
    prompt,
    setPrompt,
    pendingDelete,
    setPendingDelete,
    applyLibrary,
    handlePrompt,
    handleDelete,
    mutateBook,
    mutateVolume,
    mutateChapter,
  } = library;
  const { jobs, queueReady, enqueue } = useQueue(project);
  const structure = useStructure(project);
  const session = useEditorSession({
    project,
    chapters,
    activeChapter,
    setActiveBookId,
    storyEntries: structure.entries,
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
    preferenceCount,
  } = session;

  const activeChapterRecord = chapters.find((chapter) => chapter.id === activeChapter);
  const promptCopy = prompt
    ? prompt.mode === "rename"
      ? {
          title:
            prompt.target === "project"
              ? "重命名作品"
              : prompt.target === "book"
                ? "重命名书"
                : prompt.target === "volume"
                  ? "重命名卷"
                  : "重命名章节",
          label: "名称",
          placeholder: prompt.title ?? "",
          confirm: "保存",
        }
      : prompt.target === "project"
        ? { title: "新作品", label: "作品名称", placeholder: "例如：夜航星图", confirm: "创建" }
        : prompt.target === "book"
          ? { title: "新书", label: "书名", placeholder: "例如：雾港纪事", confirm: "创建" }
          : prompt.target === "volume"
            ? { title: "新卷", label: "卷名", placeholder: "例如：卷一 · 雾与海", confirm: "创建" }
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
            <button
              className="text-button"
              onClick={() => setPrompt({ mode: "create", target: "volume" })}
              disabled={!project || books.length === 0}
            >
              新卷
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
            const bookVolumes = volumes.filter((volume) => volume.bookId === book.id);
            const ungrouped = chapters.filter(
              (chapter) => chapter.bookId === book.id && !chapter.volumeId,
            );
            const renderChapters = (list: typeof chapters, nested: boolean) =>
              list.map((chapter, chapterIndex) => (
                <div
                  key={chapter.id}
                  className={`tree-item ${nested ? "nested" : ""} ${activeChapter === chapter.id ? "active" : ""}`}
                  onClick={() => {
                    void persistChapter();
                    setActiveChapter(chapter.id);
                    setActiveBookId(book.id);
                    setActiveVolumeId(chapter.volumeId ?? null);
                  }}
                >
                  <FileText size={14} />
                  <span>{chapter.title}</span>
                  <TreeItemActions
                    disableUp={chapterIndex === 0}
                    disableDown={chapterIndex === list.length - 1}
                    deleteTitle="删除章节"
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
              ));
            return (
              <div key={book.id} className="tree-book">
                <div
                  className={`tree-section ${activeVolumeId === null && activeChapterRecord?.bookId === book.id ? "current" : ""}`}
                  onClick={() => {
                    setActiveBookId(book.id);
                    setActiveVolumeId(null);
                  }}
                >
                  <Layers size={14} />
                  <span>{book.title}</span>
                  <TreeItemActions
                    disableUp={bookIndex === 0}
                    disableDown={bookIndex === books.length - 1}
                    deleteTitle="删除书"
                    onRename={() =>
                      setPrompt({ mode: "rename", target: "book", id: book.id, title: book.title })
                    }
                    onDelete={() => setPendingDelete({ target: "book", id: book.id, title: book.title })}
                    onMoveUp={() => void mutateBook(book.id, -1)}
                    onMoveDown={() => void mutateBook(book.id, 1)}
                  />
                </div>
                {bookVolumes.map((volume, volumeIndex) => {
                  const volumeChapters = chapters.filter((chapter) => chapter.volumeId === volume.id);
                  return (
                    <div key={volume.id} className="tree-volume">
                      <div
                        className={`tree-section volume ${activeVolumeId === volume.id ? "current" : ""}`}
                        onClick={() => {
                          setActiveBookId(book.id);
                          setActiveVolumeId(volume.id);
                        }}
                      >
                        <Bookmark size={14} />
                        <span>{volume.title}</span>
                        <TreeItemActions
                          disableUp={volumeIndex === 0}
                          disableDown={volumeIndex === bookVolumes.length - 1}
                          deleteTitle="删除卷"
                          onRename={() =>
                            setPrompt({
                              mode: "rename",
                              target: "volume",
                              id: volume.id,
                              title: volume.title,
                            })
                          }
                          onDelete={() =>
                            setPendingDelete({ target: "volume", id: volume.id, title: volume.title })
                          }
                          onMoveUp={() => void mutateVolume(volume.id, -1)}
                          onMoveDown={() => void mutateVolume(volume.id, 1)}
                        />
                      </div>
                      {renderChapters(volumeChapters, true)}
                    </div>
                  );
                })}
                {renderChapters(ungrouped, false)}
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
            <>
              <ContextRail hints={hints} />
              <ErrorBoundary label="编辑器">
                <Editor
                  key={activeChapter}
                  initialText={chapterText}
                  initialBlocks={chapterBlocks}
                  projectId={project?.id}
                  chapterId={activeChapter}
                  onTextChange={(text) => {
                    draftText.current = text;
                  }}
                  onNearbyChange={(nearby) => {
                    refreshHints(nearby.current, nearby.previous);
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
            </>
          )}

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
            className={sidebarTab === "structure" ? "active" : ""}
            onClick={() => setSidebarTab("structure")}
          >
            <BookOpen size={14} />
            结构
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
            <h3>当前场景包</h3>
            <div className="context-card">
              <div className="context-card-title">
                <CircleDot size={12} />
                POV 边界
              </div>
              <p>打开章节后，编辑器上方会按当前段落匹配你预先写好的人物、设定和伏笔。</p>
            </div>
            <div className="context-card">
              <div className="context-card-title">
                <CircleDot size={12} />
                作品结构
              </div>
              <p>作品 → 书 → 可选卷 → 章。卷只用来分组；删卷不会删章节。</p>
            </div>
          </div>
        )}

        {sidebarTab === "structure" && (
          <StructurePanel
            disabled={!project}
            busy={structure.busy}
            error={structure.error}
            entries={structure.entries}
            onCreate={(kind, title, summary) => void structure.create(kind, title, summary)}
            onDelete={(entry) => void structure.remove(entry)}
          />
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
              {preferenceCount > 0 && (
                <p>已记住 {preferenceCount} 条写作偏好，下次续写会写进提示。</p>
              )}
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
          setModelConfig({
            ...config,
            apiKey: "",
            apiKeySet: Boolean(config.apiKey) || Boolean(config.apiKeySet),
          });
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
              : pendingDelete?.target === "volume"
                ? "删除卷"
                : "删除章节"
        }
        body={
          pendingDelete?.target === "volume"
            ? `确定删除「${pendingDelete.title}」？卷下的章节会留在书里，只是不再分卷。`
            : `确定删除「${pendingDelete?.title ?? ""}」？此操作不可撤销。`
        }
        onClose={() => setPendingDelete(null)}
        onConfirm={handleDelete}
      />
    </div>
  );
}
