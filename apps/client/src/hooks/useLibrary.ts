import { useCallback, useEffect, useState } from "react";
import { libraryApi } from "../api";
import { Book, Chapter, LibrarySnapshot, Project } from "../types";
import { logger } from "../logger";

export type PromptKind =
  | { mode: "create" | "rename"; target: "project" | "book" | "chapter"; id?: string; title?: string }
  | null;
export type DeleteKind = { target: "project" | "book" | "chapter"; id: string; title: string } | null;

export function useLibrary() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [project, setProject] = useState<Project | null>(null);
  const [books, setBooks] = useState<Book[]>([]);
  const [chapters, setChapters] = useState<Chapter[]>([]);
  const [activeChapter, setActiveChapter] = useState<string | null>(null);
  const [activeBookId, setActiveBookId] = useState<string | null>(null);
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

  return {
    projects,
    project,
    books,
    chapters,
    activeChapter,
    setActiveChapter,
    activeBookId,
    setActiveBookId,
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
    mutateChapter,
  };
}
