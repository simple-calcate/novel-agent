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
  History,
} from "lucide-react";
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
import { SceneStrip } from "./components/SceneStrip";
import { PreferencePanel } from "./components/PreferencePanel";
import { PluginModal } from "./components/PluginModal";
import { HistoryPanel } from "./components/HistoryPanel";
import { logger } from "./logger";
import { useLibrary } from "./hooks/useLibrary";
import { useQueue } from "./hooks/useQueue";
import { useEditorSession } from "./hooks/useEditorSession";
import { useStructure } from "./hooks/useStructure";
import { PluginSummary } from "./types";
import { uniqueNames } from "./plugins/format";
import { useI18n } from "./i18n";

export function App() {
  const { t } = useI18n();
  const [sidebarTab, setSidebarTab] = useState<"context" | "structure" | "workflow" | "agent">(
    "structure",
  );
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [logPanelOpen, setLogPanelOpen] = useState(false);
  const [pluginOpen, setPluginOpen] = useState(false);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [plugins, setPlugins] = useState<PluginSummary[]>([]);
  const library = useLibrary();
  const {
    projects,
    project,
    books,
    volumes,
    chapters,
    scenes,
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
    mutateScene,
    setScenePov,
    openSampleChapter,
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
    preferences,
    togglePreference,
    hintPrefs,
    pinHint,
    ignoreHint,
    editorNonce,
    applyChapterBody,
  } = session;

  const pluginCharacterNames = uniqueNames([
    ...hints.filter((hint) => hint.kind === "characterState").map((hint) => hint.title),
    ...structure.entries.filter((entry) => entry.kind === "character").map((entry) => entry.title),
  ]);

  async function handleOpenSample(saveCurrent: boolean) {
    if (saveCurrent) await persistChapter();
    const installed = await openSampleChapter();
    if (installed) await structure.refresh(installed.projectId);
  }

  const activeChapterRecord = chapters.find((chapter) => chapter.id === activeChapter);
  const promptCopy = prompt
    ? prompt.mode === "rename"
      ? {
          title:
            prompt.target === "project"
              ? t("library.renameProject")
              : prompt.target === "book"
                ? t("library.renameBook")
                : prompt.target === "volume"
                  ? t("library.renameVolume")
                  : prompt.target === "scene"
                    ? t("library.renameScene")
                    : t("library.renameChapter"),
          label: t("common.name"),
          placeholder: prompt.title ?? "",
          confirm: t("common.save"),
        }
      : prompt.target === "project"
        ? {
            title: t("library.newProject"),
            label: t("library.projectName"),
            placeholder: t("library.projectPlaceholder"),
            confirm: t("common.create"),
          }
        : prompt.target === "book"
          ? {
              title: t("library.newBook"),
              label: t("library.bookName"),
              placeholder: t("library.bookPlaceholder"),
              confirm: t("common.create"),
            }
          : prompt.target === "volume"
            ? {
                title: t("library.newVolume"),
                label: t("library.volumeName"),
                placeholder: t("library.volumePlaceholder"),
                confirm: t("common.create"),
              }
            : prompt.target === "scene"
              ? {
                  title: t("library.newScene"),
                  label: t("library.sceneTitle"),
                  placeholder: t("library.scenePlaceholder"),
                  confirm: t("common.create"),
                }
              : {
                  title: t("library.newChapter"),
                  label: t("library.chapterTitle"),
                  placeholder: t("library.chapterPlaceholder"),
                  confirm: t("common.create"),
                }
    : { title: "", label: "", placeholder: "", confirm: t("common.create") };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <PenLine size={20} />
          </div>
          <div>
            <div className="brand-name">{t("meta.brandName")}</div>
            <div className="brand-sub">{t("meta.brandSub")}</div>
          </div>
        </div>

        <div className="project-card">
          <div className="project-label">{t("library.currentProject")}</div>
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
            <div className="project-title">{t("library.noProject")}</div>
          )}
          <div className="project-meta">
            <span>{t("library.localFirst")}</span>
            <span>·</span>
            <span>{t("library.bookCount", { count: books.length })}</span>
          </div>
          <div className="project-actions">
            <button className="text-button" onClick={() => setPrompt({ mode: "create", target: "project" })}>
              {t("library.newProject")}
            </button>
            <button className="text-button" onClick={() => setPrompt({ mode: "create", target: "book" })}>
              {t("library.newBook")}
            </button>
            <button
              className="text-button"
              onClick={() => setPrompt({ mode: "create", target: "volume" })}
              disabled={!project || books.length === 0}
            >
              {t("library.newVolume")}
            </button>
            {project && (
              <>
                <button
                  className="text-button"
                  onClick={() =>
                    setPrompt({ mode: "rename", target: "project", id: project.id, title: project.title })
                  }
                >
                  {t("common.rename")}
                </button>
                <button
                  className="text-button"
                  onClick={() => setPendingDelete({ target: "project", id: project.id, title: project.title })}
                >
                  {t("common.delete")}
                </button>
              </>
            )}
          </div>
        </div>

        <nav className="tree">
          {libraryError && <div className="tree-empty">{libraryError}</div>}
          {books.length === 0 && (
            <div className="tree-empty">{t("library.emptyTree")}</div>
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
                  className={`tree-item ${nested ? "nested" : "ungrouped"} ${activeChapter === chapter.id ? "active" : ""}`}
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
                    deleteTitle={t("library.deleteChapter")}
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
                    deleteTitle={t("library.deleteBook")}
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
                          deleteTitle={t("library.deleteVolume")}
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
                {ungrouped.length > 0 && bookVolumes.length > 0 && (
                  <div className="tree-section ungrouped-label">{t("library.ungrouped")}</div>
                )}
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
            <span>{t("library.newChapter")}</span>
          </button>
          <button
            className="tree-item add"
            onClick={() => {
              void handleOpenSample(true);
            }}
          >
            <BookOpen size={14} />
            <span>{t("library.openSample")}</span>
          </button>
        </nav>

        <div className="sidebar-footer">
          <button className="icon-button" title={t("chrome.settings")} onClick={() => setSettingsOpen(true)}>
            <Settings size={16} />
          </button>
          <button
            className={`icon-button ${pluginOpen ? "active" : ""}`}
            title={t("chrome.plugins")}
            onClick={() => {
              setPluginOpen(true);
              void libraryApi.listPlugins().then(setPlugins).catch(() => setPlugins([]));
            }}
          >
            <Layers size={16} />
          </button>
          <button className="icon-button" title={t("chrome.queue")}>
            <ListChecks size={16} />
          </button>
          <button
            className={`icon-button ${logPanelOpen ? "active" : ""}`}
            title={t("chrome.logs")}
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
            <span>{activeChapterRecord?.title ?? t("library.noChapter")}</span>
            <button
              className={`revision-badge ${historyOpen ? "active" : ""}`}
              title={t("chrome.history")}
              disabled={!activeChapter}
              onClick={() => {
                void persistChapter().then(() => setHistoryOpen(true));
              }}
            >
              <History size={12} />
              R{revision}
            </button>
          </div>
          <div className="topbar-actions">
            <button
              className="action-button ghost"
              disabled={!project}
              onClick={() => enqueue("continuity.check")}
            >
              <CheckCircle2 size={14} />
              {t("chrome.check")}
            </button>
            <button className="action-button primary" onClick={handleGenerate}>
              <Sparkles size={14} />
              {t("chrome.continue")}
            </button>
          </div>
        </header>

        <div className="editor-area">
          {!activeChapter && (
            <div className="workspace-empty">
              <p>{t("chrome.emptyWorkspace")}</p>
              <div className="workspace-empty-actions">
                <button className="btn primary" onClick={() => void handleOpenSample(false)}>
                  {t("library.openSample")}
                </button>
                <button className="btn" onClick={() => setPrompt({ mode: "create", target: "book" })}>
                  {t("chrome.createBook")}
                </button>
                <button className="btn" onClick={() => setPrompt({ mode: "create", target: "project" })}>
                  {t("chrome.createProjectOnly")}
                </button>
              </div>
            </div>
          )}
          {activeChapter && chapterReady && (
            <>
              <ContextRail
                hints={hints}
                pinnedIds={hintPrefs.pinned}
                ignoredIds={hintPrefs.ignored}
                onPin={pinHint}
                onIgnore={ignoreHint}
              />
              {project && activeChapter && (
                <SceneStrip
                  scenes={scenes.filter((scene) => scene.chapterId === activeChapter)}
                  characters={structure.entries.filter((entry) => entry.kind === "character")}
                  disabled={!project}
                  onCreate={() => setPrompt({ mode: "create", target: "scene" })}
                  onRename={(scene) =>
                    setPrompt({ mode: "rename", target: "scene", id: scene.id, title: scene.title })
                  }
                  onDelete={(scene) =>
                    setPendingDelete({ target: "scene", id: scene.id, title: scene.title })
                  }
                  onMove={(sceneId, delta) => void mutateScene(sceneId, delta)}
                  onPov={(scene, povEntryId) => void setScenePov(scene.id, povEntryId)}
                />
              )}
              <ErrorBoundary label={t("chrome.editorLabel")}>
                <Editor
                  key={`${activeChapter}:${editorNonce}`}
                  initialText={chapterText}
                  initialBlocks={chapterBlocks}
                  projectId={project?.id}
                  chapterId={activeChapter}
                  chapterTitle={activeChapterRecord?.title}
                  onTextChange={(text) => {
                    draftText.current = text;
                  }}
                  onNearbyChange={(nearby) => {
                    refreshHints(nearby.current, nearby.previous);
                  }}
                  onBlocksChange={(blocks) => {
                    draftBlocks.current = blocks;
                  }}
                  storyEntries={structure.entries}
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
            {t("chrome.tabContext")}
          </button>
          <button
            className={sidebarTab === "structure" ? "active" : ""}
            onClick={() => setSidebarTab("structure")}
          >
            <BookOpen size={14} />
            {t("chrome.tabStructure")}
          </button>
          <button
            className={sidebarTab === "workflow" ? "active" : ""}
            onClick={() => setSidebarTab("workflow")}
          >
            <ListChecks size={14} />
            {t("chrome.tabWorkflow")}
          </button>
          <button
            className={sidebarTab === "agent" ? "active" : ""}
            onClick={() => setSidebarTab("agent")}
          >
            <MessageSquare size={14} />
            {t("chrome.tabAgent")}
          </button>
        </div>

        {sidebarTab === "context" && (
          <div className="panel-content">
            <h3>{t("context.heading")}</h3>
            <div className="context-card">
              <div className="context-card-title">
                <CircleDot size={12} />
                {t("context.step1Title")}
              </div>
              <p>{t("context.step1Body")}</p>
            </div>
            <div className="context-card">
              <div className="context-card-title">
                <CircleDot size={12} />
                {t("context.step2Title")}
              </div>
              <p>{t("context.step2Body")}</p>
            </div>
            <div className="context-card">
              <div className="context-card-title">
                <CircleDot size={12} />
                {t("context.step3Title")}
              </div>
              <p>{t("context.step3Body")}</p>
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
            chapterText={chapterText}
            characterNames={pluginCharacterNames}
            onRun={(operation, label) => {
              logger.info("手动运行工作流", { operation, label });
              enqueue(operation);
            }}
          />
        )}

        {sidebarTab === "agent" && (
          <div className="panel-content">
            <h3>{t("agent.heading")}</h3>
            <div className="agent-message">
              <strong>{t("agent.system")}</strong>
              <p>
                {project
                  ? t("agent.withProject", { title: project.title, revision })
                  : t("agent.withoutProject")}
              </p>
              {preferences.length > 0 && (
                <p>
                  {t("agent.remembered", {
                    count: preferences.filter((item) => item.status !== "disabled").length,
                  })}
                </p>
              )}
            </div>
            <PreferencePanel rules={preferences} onToggle={(rule, disabled) => void togglePreference(rule, disabled)} />
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
            await libraryApi.saveModelConfig(config);
            logger.info("配置已同步到后端");
          } catch (e) {
            logger.error("保存失败", { error: String(e) });
          }
        }}
      />

      <LogPanel open={logPanelOpen} onClose={() => setLogPanelOpen(false)} />

      <HistoryPanel
        open={historyOpen}
        chapterId={activeChapter}
        chapterTitle={activeChapterRecord?.title}
        currentRevision={revision}
        onClose={() => setHistoryOpen(false)}
        onRestore={async (target) => {
          if (!activeChapter) return;
          await persistChapter();
          const body = await libraryApi.restoreChapterRevision(activeChapter, target);
          applyChapterBody(body);
        }}
      />

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
            ? t("library.deleteProject")
            : pendingDelete?.target === "book"
              ? t("library.deleteBook")
              : pendingDelete?.target === "volume"
                ? t("library.deleteVolume")
                : pendingDelete?.target === "scene"
                  ? t("library.deleteScene")
                  : t("library.deleteChapter")
        }
        body={
          pendingDelete?.target === "volume"
            ? t("library.confirmDeleteVolume", { title: pendingDelete.title })
            : pendingDelete?.target === "scene"
              ? t("library.confirmDeleteScene", { title: pendingDelete.title })
              : t("library.confirmDelete", { title: pendingDelete?.title ?? "" })
        }
        onClose={() => setPendingDelete(null)}
        onConfirm={handleDelete}
      />
      <PluginModal
        open={pluginOpen}
        plugins={plugins}
        chapterText={session.chapterText}
        characterNames={pluginCharacterNames}
        onClose={() => setPluginOpen(false)}
      />
    </div>
  );
}
