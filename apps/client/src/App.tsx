import { useCallback, useEffect, useRef, useState } from "react";
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
import { listen } from "@tauri-apps/api/event";
import { libraryApi } from "./api";
import { Book, Chapter, CommandResult, ContentBlock, ContextHint, JobView, Project } from "./types";
import { Editor, AIPreview } from "./components/Editor";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { ContextRail } from "./components/ContextRail";
import { WorkflowPanel } from "./components/WorkflowPanel";
import { SettingsModal, ModelConfig } from "./components/SettingsModal";
import { LogPanel } from "./components/LogPanel";
import { CreateDialog } from "./components/CreateDialog";
import { ConfirmDialog, TreeItemActions } from "./components/LibraryActions";
import { logger } from "./logger";

type PromptKind =
  | { mode: "create" | "rename"; target: "project" | "book" | "chapter"; id?: string; title?: string }
  | null;
type DeleteKind = { target: "project" | "book" | "chapter"; id: string; title: string } | null;

export function App() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [project, setProject] = useState<Project | null>(null);
  const [books, setBooks] = useState<Book[]>([]);
  const [chapters, setChapters] = useState<Chapter[]>([]);
  const [activeChapter, setActiveChapter] = useState<string | null>(null);
  const [activeBookId, setActiveBookId] = useState<string | null>(null);
  const [chapterText, setChapterText] = useState("");
  const [chapterBlocks, setChapterBlocks] = useState<ContentBlock[]>([]);
  const [chapterReady, setChapterReady] = useState(false);
  const [hints, setHints] = useState<ContextHint[]>([]);
  const [aiPreview, setAiPreview] = useState("");
  const [sidebarTab, setSidebarTab] = useState<"context" | "workflow" | "agent">("context");
  const [jobs, setJobs] = useState<Array<{ id: string; label: string; status: string }>>([]);
  const [revision, setRevision] = useState(0);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [logPanelOpen, setLogPanelOpen] = useState(false);
  const [_modelConfig, setModelConfig] = useState<ModelConfig | null>(null);
  const [queueReady, setQueueReady] = useState(false);
  const [prompt, setPrompt] = useState<PromptKind>(null);
  const [pendingDelete, setPendingDelete] = useState<DeleteKind>(null);
  const [libraryError, setLibraryError] = useState<string | null>(null);

  const draftText = useRef("");
  const draftBlocks = useRef<ContentBlock[]>([]);

  const applyLibrary = useCallback(
    (snapshot: {
      projects: Project[];
      activeProjectId?: string | null;
      books: Book[];
      chapters: Chapter[];
    }) => {
      setProjects(snapshot.projects);
      const current =
        snapshot.projects.find((item) => item.id === snapshot.activeProjectId) ??
        snapshot.projects[0] ??
        null;
      setProject(current);
      setBooks(snapshot.books);
      setChapters(snapshot.chapters);
      setActiveChapter((previous) => {
        if (previous && snapshot.chapters.some((chapter) => chapter.id === previous)) {
          return previous;
        }
        return snapshot.chapters[0]?.id ?? null;
      });
      setActiveBookId((previous) => {
        if (previous && snapshot.books.some((book) => book.id === previous)) {
          return previous;
        }
        return snapshot.books[0]?.id ?? snapshot.chapters[0]?.bookId ?? null;
      });
    },
    [],
  );

  const refreshLibrary = useCallback(
    async (projectId?: string) => {
      try {
        const snapshot = await libraryApi.loadLibrary(projectId);
        applyLibrary(snapshot);
        setLibraryError(null);
      } catch (error) {
        logger.error("加载作品库失败", { error: String(error) });
        setLibraryError(String(error));
      }
    },
    [applyLibrary],
  );

  useEffect(() => {
    void refreshLibrary();
  }, [refreshLibrary]);

  useEffect(() => {
    invoke<ModelConfig | null>("load_model_config")
      .then((config) => {
        if (config) {
          logger.info("恢复模型配置", { provider: config.provider, model: config.model });
          setModelConfig(config);
        }
      })
      .catch((e) => logger.error("加载配置失败", { error: String(e) }));
  }, []);

  useEffect(() => {
    if (!activeChapter) {
      setChapterText("");
      setChapterReady(false);
      setRevision(0);
      draftText.current = "";
      return;
    }
    let cancelled = false;
    setChapterReady(false);
    libraryApi
      .loadChapter(activeChapter)
      .then((body) => {
        if (cancelled) return;
        setChapterText(body.text);
        setChapterBlocks(body.blocks ?? []);
        draftText.current = body.text;
        draftBlocks.current = body.blocks ?? [];
        setRevision(body.revision);
        setChapterReady(true);
        const chapter = chapters.find((item) => item.id === activeChapter);
        if (chapter) setActiveBookId(chapter.bookId);
      })
      .catch((error) => {
        if (cancelled) return;
        logger.error("加载章节失败", { error: String(error) });
        setChapterText("");
        setChapterReady(true);
      });
    return () => {
      cancelled = true;
    };
  }, [activeChapter]);

  const jobsRef = useRef(jobs);
  jobsRef.current = jobs;
  const drainingRef = useRef(false);

  const refreshJobs = useCallback(async () => {
    try {
      const result = await invoke<CommandResult<JobView[]>>("list_jobs");
      if (result?.ok && result.data) {
        setJobs(
          result.data.map((job) => ({
            id: job.id,
            label: operationLabel(job.operation),
            status: job.status,
          })),
        );
        setQueueReady(true);
      }
    } catch (e) {
      logger.warn("任务列表拉取失败（可能在纯浏览器预览中）", { error: String(e) });
    }
  }, []);

  const drainQueue = useCallback(async () => {
    if (drainingRef.current) return;
    drainingRef.current = true;
    try {
      for (let i = 0; i < 50; i++) {
        const step = await invoke<CommandResult<{ executed: boolean }>>("run_queue_step");
        if (!step?.ok || !step.data?.executed) break;
        await refreshJobs();
      }
    } catch (e) {
      logger.warn("队列驱动失败（可能在纯浏览器预览中）", { error: String(e) });
    } finally {
      drainingRef.current = false;
    }
  }, [refreshJobs]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen("queue:changed", () => {
      void drainQueue();
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => logger.warn("队列事件监听不可用（纯浏览器预览）", { error: String(e) }));

    void drainQueue();

    const timer = setInterval(() => {
      const hasWork = jobsRef.current.some(
        (j) => j.status === "pending" || j.status === "running",
      );
      if (hasWork) void drainQueue();
    }, 30_000);
    return () => {
      unlisten?.();
      clearInterval(timer);
    };
  }, [drainQueue]);

  const enqueue = useCallback(
    async (operation: string, extraPayload?: Record<string, unknown>) => {
      if (!project) {
        logger.warn("未选择作品，跳过入队", { operation });
        return;
      }
      logger.info("入队任务", { operation });
      const payload = { projectId: project.id, ...extraPayload };
      try {
        const result = await invoke<CommandResult<{ jobId: string }>>("enqueue_job", {
          input: { projectId: project.id, operation, payload, priority: 0 },
        });
        if (!result.ok) {
          logger.error("入队失败", { operation, error: result.error });
        } else {
          void drainQueue();
        }
      } catch (e) {
        logger.error("入队异常", { operation, error: String(e) });
      }
    },
    [project, drainQueue],
  );

  const persistChapter = useCallback(async () => {
    if (!activeChapter) return;
    try {
      const saved = await libraryApi.saveChapter(
        activeChapter,
        draftText.current,
        draftBlocks.current,
      );
      setRevision(saved.revision);
    } catch (error) {
      logger.warn("保存章节失败", { error: String(error) });
    }
  }, [activeChapter]);

  const refreshHints = useCallback(
    async (nearbyText: string) => {
      if (!project || !activeChapter) return;
      logger.debug("刷新上下文提示", { revision, textLength: nearbyText.length });
      try {
        const result = await invoke<CommandResult<ContextHint[]>>("context_hints", {
          input: {
            projectId: project.id,
            chapterId: activeChapter,
            revision,
            nearbyText,
            generation: Date.now(),
          },
        });
        if (result.ok && result.data) {
          logger.info("上下文提示更新", { count: result.data.length });
          setHints(result.data);
        }
      } catch (e) {
        logger.error("上下文提示失败", { error: String(e) });
        setHints(buildLocalHints(nearbyText, revision));
      }
    },
    [project, activeChapter, revision],
  );

  useEffect(() => {
    setHints(buildLocalHints("", revision));
  }, [revision]);

  const handleGenerate = useCallback(async () => {
    if (!_modelConfig) {
      logger.warn("未配置模型");
      setAiPreview("请先点击左下角「设置」配置 AI 模型");
      return;
    }
    if (!activeChapter) {
      setAiPreview("请先创建并打开一个章节");
      return;
    }
    logger.info("开始 AI 续写", { provider: _modelConfig.provider, model: _modelConfig.model });
    try {
      const result = await invoke<{ operations: Array<{ text: string }> }>(
        "generate_continuation",
        {
          chapterId: activeChapter,
          revision,
          prompt: "继续当前剧情",
          contextText: draftText.current.slice(-800),
          config: _modelConfig,
        },
      );
      if (result?.operations?.[0]) {
        logger.info("AI 续写成功", { length: result.operations[0].text.length });
        setAiPreview(result.operations[0].text);
      }
    } catch (e) {
      logger.error("AI 续写失败", { error: String(e) });
      setAiPreview(`调用失败: ${e}`);
    }
  }, [_modelConfig, activeChapter, revision]);

  const handleAccept = useCallback(() => {
    if (aiPreview && (window as any).__editorInsert) {
      (window as any).__editorInsert(aiPreview);
      logger.info("接受 AI 续写");
      setAiPreview("");
    }
  }, [aiPreview]);

  const handleReject = useCallback(() => {
    logger.info("拒绝 AI 续写");
    setAiPreview("");
  }, []);

  const handlePrompt = useCallback(
    async (title: string) => {
      if (!prompt) return;
      if (prompt.mode === "create") {
        if (prompt.target === "project") {
          const created = await libraryApi.createProject(title);
          await refreshLibrary(created.id);
          return;
        }
        if (prompt.target === "book") {
          let owner = project;
          if (!owner) {
            owner = await libraryApi.createProject(title);
          }
          const book = await libraryApi.createBook(owner.id, title);
          await refreshLibrary(owner.id);
          setActiveBookId(book.id);
          return;
        }
        if (!project) {
          throw new Error("请先创建作品或书籍");
        }
        const bookId = activeBookId ?? books[0]?.id;
        if (!bookId) {
          throw new Error("请先创建一本书");
        }
        const chapter = await libraryApi.createChapter(project.id, bookId, title);
        await refreshLibrary(project.id);
        setActiveChapter(chapter.id);
        setActiveBookId(bookId);
        return;
      }
      if (!project && prompt.target !== "project") {
        throw new Error("未选择作品");
      }
      if (prompt.target === "project" && prompt.id) {
        applyLibrary(await libraryApi.renameProject(prompt.id, title));
        return;
      }
      if (!project || !prompt.id) return;
      if (prompt.target === "book") {
        applyLibrary(await libraryApi.renameBook(project.id, prompt.id, title));
        return;
      }
      applyLibrary(await libraryApi.renameChapter(project.id, prompt.id, title));
    },
    [prompt, project, activeBookId, books, refreshLibrary, applyLibrary],
  );

  const handleDelete = useCallback(async () => {
    if (!pendingDelete) return;
    if (pendingDelete.target === "project") {
      applyLibrary(await libraryApi.deleteProject(pendingDelete.id));
      return;
    }
    if (!project) return;
    if (pendingDelete.target === "book") {
      applyLibrary(await libraryApi.deleteBook(project.id, pendingDelete.id));
      return;
    }
    applyLibrary(await libraryApi.deleteChapter(project.id, pendingDelete.id));
  }, [pendingDelete, project, applyLibrary]);

  const mutateBook = useCallback(
    async (bookId: string, delta: number) => {
      if (!project) return;
      applyLibrary(await libraryApi.moveBook(project.id, bookId, delta));
    },
    [project, applyLibrary],
  );

  const mutateChapter = useCallback(
    async (chapterId: string, delta: number) => {
      if (!project) return;
      applyLibrary(await libraryApi.moveChapter(project.id, chapterId, delta));
    },
    [project, applyLibrary],
  );

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
        initialConfig={_modelConfig}
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

const OPERATION_LABELS: Record<string, string> = {
  "document.save": "保存文档",
  "index.rebuild": "刷新索引",
  "continuity.check": "连续性检查",
  "backup.create": "创建备份",
  "agent.continuation": "AI 续写",
  "agent.run": "运行 Agent",
  "plugin.operation": "插件操作",
  "block.save": "保存块",
  "block.edit": "编辑块",
  "training.export": "导出训练数据",
};

function operationLabel(operation: string): string {
  return OPERATION_LABELS[operation] ?? operation;
}

function buildLocalHints(nearby: string, revision: number): ContextHint[] {
  const base: ContextHint[] = [];
  if (nearby.includes("玺")) {
    base.push({
      id: "h0",
      kind: "plotHook",
      title: "旧王玺",
      summary: "沈雾不知道它已在船长手中；避免提前揭示",
      sourceLabel: "正史",
      matchReason: "当前文字包含「玺」",
      confidence: 0.98,
      score: 1,
      generation: revision,
      revision,
    });
  }
  return base.slice(0, 5);
}
