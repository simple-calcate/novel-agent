import { useCallback, useEffect, useState } from "react";
import { libraryApi } from "../api";
import { Book, Chapter, LibrarySnapshot, Project, Scene, Volume } from "../types";
import { logger } from "../logger";
import { installSampleChapter } from "../editor/sampleChapter";

export type PromptKind =
  | {
      mode: "create" | "rename";
      target: "project" | "book" | "volume" | "chapter" | "scene";
      id?: string;
      title?: string;
    }
  | null;
export type DeleteKind =
  | { target: "project" | "book" | "volume" | "chapter" | "scene"; id: string; title: string }
  | null;

export function useLibrary() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [project, setProject] = useState<Project | null>(null);
  const [books, setBooks] = useState<Book[]>([]);
  const [volumes, setVolumes] = useState<Volume[]>([]);
  const [chapters, setChapters] = useState<Chapter[]>([]);
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [activeChapter, setActiveChapter] = useState<string | null>(null);
  const [activeBookId, setActiveBookId] = useState<string | null>(null);
  const [activeVolumeId, setActiveVolumeId] = useState<string | null>(null);
  const [libraryError, setLibraryError] = useState<string | null>(null);
  const [prompt, setPrompt] = useState<PromptKind>(null);
  const [pendingDelete, setPendingDelete] = useState<DeleteKind>(null);

  const applyLibrary = useCallback((snapshot: LibrarySnapshot) => {
    setProjects(snapshot.projects);
    const current =
      snapshot.projects.find((item) => item.id === snapshot.activeProjectId) ??
      snapshot.projects[0] ??
      null;
    setProject(current);
    setBooks(snapshot.books);
    setVolumes(snapshot.volumes ?? []);
    setChapters(snapshot.chapters);
    setScenes(snapshot.scenes ?? []);
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
    setActiveVolumeId((previous) => {
      const listed = snapshot.volumes ?? [];
      if (previous && listed.some((volume) => volume.id === previous)) {
        return previous;
      }
      return null;
    });
  }, []);

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
          setActiveVolumeId(null);
          return;
        }
        if (prompt.target === "volume") {
          if (!project) throw new Error("请先创建作品");
          const bookId = activeBookId ?? books[0]?.id;
          if (!bookId) throw new Error("请先创建一本书");
          const volume = await libraryApi.createVolume(project.id, bookId, title);
          await refreshLibrary(project.id);
          setActiveBookId(bookId);
          setActiveVolumeId(volume.id);
          return;
        }
        if (prompt.target === "scene") {
          if (!project) throw new Error("请先创建作品");
          if (!activeChapter) throw new Error("请先打开一章");
          await libraryApi.createScene(project.id, activeChapter, title);
          await refreshLibrary(project.id);
          return;
        }
        if (!project) {
          throw new Error("请先创建作品或书籍");
        }
        const bookId = activeBookId ?? books[0]?.id;
        if (!bookId) {
          throw new Error("请先创建一本书");
        }
        const volumeId =
          activeVolumeId && volumes.some((volume) => volume.id === activeVolumeId && volume.bookId === bookId)
            ? activeVolumeId
            : null;
        const chapter = await libraryApi.createChapter(project.id, bookId, title, volumeId);
        await refreshLibrary(project.id);
        setActiveChapter(chapter.id);
        setActiveBookId(bookId);
        if (volumeId) setActiveVolumeId(volumeId);
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
      if (prompt.target === "volume") {
        applyLibrary(await libraryApi.renameVolume(project.id, prompt.id, title));
        return;
      }
      if (prompt.target === "scene") {
        applyLibrary(await libraryApi.renameScene(project.id, prompt.id, title));
        return;
      }
      applyLibrary(await libraryApi.renameChapter(project.id, prompt.id, title));
    },
    [prompt, project, activeBookId, activeVolumeId, activeChapter, books, volumes, refreshLibrary, applyLibrary],
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
    if (pendingDelete.target === "volume") {
      applyLibrary(await libraryApi.deleteVolume(project.id, pendingDelete.id));
      return;
    }
    if (pendingDelete.target === "scene") {
      applyLibrary(await libraryApi.deleteScene(project.id, pendingDelete.id));
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

  const mutateVolume = useCallback(
    async (volumeId: string, delta: number) => {
      if (!project) return;
      applyLibrary(await libraryApi.moveVolume(project.id, volumeId, delta));
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

  const mutateScene = useCallback(
    async (sceneId: string, delta: number) => {
      if (!project) return;
      applyLibrary(await libraryApi.moveScene(project.id, sceneId, delta));
    },
    [project, applyLibrary],
  );

  const setScenePov = useCallback(
    async (sceneId: string, povEntryId: string | null) => {
      if (!project) return;
      applyLibrary(await libraryApi.setScenePov(project.id, sceneId, povEntryId));
    },
    [project, applyLibrary],
  );

  const openSampleChapter = useCallback(async () => {
    try {
      const installed = await installSampleChapter();
      const snapshot = await libraryApi.setActiveProject(installed.projectId);
      applyLibrary(snapshot);
      setActiveBookId(installed.bookId);
      setActiveVolumeId(null);
      setActiveChapter(installed.chapterId);
      setLibraryError(null);
    } catch (error) {
      logger.error("打开示例章节失败", { error: String(error) });
      setLibraryError(String(error));
    }
  }, [applyLibrary]);

  return {
    projects,
    project,
    books,
    volumes,
    chapters,
    scenes,
    activeChapter,
    setActiveChapter,
    activeBookId,
    setActiveBookId,
    activeVolumeId,
    setActiveVolumeId,
    libraryError,
    prompt,
    setPrompt,
    pendingDelete,
    setPendingDelete,
    applyLibrary,
    refreshLibrary,
    handlePrompt,
    handleDelete,
    mutateBook,
    mutateVolume,
    mutateChapter,
    mutateScene,
    setScenePov,
    openSampleChapter,
  };
}
