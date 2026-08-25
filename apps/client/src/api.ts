/**
 * 客户端对宿主 IPC 的唯一入口。
 *
 * 桌面（Tauri）走命令层；浏览器预览走内存实现，便于无后端时开发 UI。
 * 新增作品/书/章能力时，先补这里再改界面。
 */
import { invoke } from "@tauri-apps/api/core";
import {
  Book,
  Chapter,
  ChapterBody,
  CommandResult,
  ContentBlock,
  LibrarySnapshot,
  Project,
} from "./types";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  const result = await invoke<CommandResult<T>>(name, args);
  if (!result?.ok) {
    throw new Error(result?.error ?? `${name} failed`);
  }
  return result.data as T;
}

interface MemoryState {
  projects: Project[];
  books: Book[];
  chapters: Chapter[];
  texts: Record<string, ChapterBody>;
  activeProjectId: string | undefined;
}

const memory: MemoryState = {
  projects: [],
  books: [],
  chapters: [],
  texts: {},
  activeProjectId: undefined,
};

function nowIso(): string {
  return new Date().toISOString();
}

function newId(): string {
  return crypto.randomUUID();
}

function snapshot(projectId?: string): LibrarySnapshot {
  const active =
    projectId ?? memory.activeProjectId ?? memory.projects[0]?.id ?? undefined;
  memory.activeProjectId = active;
  return {
    projects: memory.projects,
    activeProjectId: active ?? null,
    books: memory.books.filter((book) => book.projectId === active),
    chapters: memory.chapters.filter((chapter) =>
      memory.books.some((book) => book.id === chapter.bookId && book.projectId === active),
    ),
  };
}

/** 仅测试用：清空浏览器内存库。 */
export function resetMemoryLibrary(): void {
  memory.projects = [];
  memory.books = [];
  memory.chapters = [];
  memory.texts = {};
  memory.activeProjectId = undefined;
}

export const libraryApi = {
  async loadLibrary(projectId?: string | null): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("load_library", { projectId: projectId ?? null });
    }
    return snapshot(projectId ?? undefined);
  },

  async setActiveProject(projectId: string): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("set_active_project", { projectId });
    }
    memory.activeProjectId = projectId;
    return snapshot(projectId);
  },

  async createProject(title: string): Promise<Project> {
    if (isTauriRuntime()) {
      return command<Project>("create_project", { input: { title } });
    }
    const project: Project = {
      id: newId(),
      title,
      createdAt: nowIso(),
      updatedAt: nowIso(),
    };
    memory.projects.unshift(project);
    memory.activeProjectId = project.id;
    return project;
  },

  async createBook(projectId: string, title: string, synopsis = ""): Promise<Book> {
    if (isTauriRuntime()) {
      return command<Book>("create_book", {
        input: { projectId, title, synopsis, position: 0 },
      });
    }
    const siblings = memory.books.filter((book) => book.projectId === projectId);
    const book: Book = {
      id: newId(),
      projectId,
      title,
      synopsis,
      position: siblings.reduce((max, item) => Math.max(max, item.position), 0) + 1,
    };
    memory.books.push(book);
    return book;
  },

  async createChapter(projectId: string, bookId: string, title: string): Promise<Chapter> {
    if (isTauriRuntime()) {
      return command<Chapter>("create_chapter", {
        input: { projectId, bookId, title, position: 0 },
      });
    }
    const siblings = memory.chapters.filter((chapter) => chapter.bookId === bookId);
    const chapter: Chapter = {
      id: newId(),
      bookId,
      title,
      position: siblings.reduce((max, item) => Math.max(max, item.position), 0) + 1,
      currentRevision: 0,
      status: "draft",
    };
    memory.chapters.push(chapter);
    memory.texts[chapter.id] = { chapterId: chapter.id, revision: 0, text: "", blocks: [] };
    return chapter;
  },

  async loadChapter(chapterId: string): Promise<ChapterBody> {
    if (isTauriRuntime()) {
      return command<ChapterBody>("load_chapter", { chapterId });
    }
    const stored = memory.texts[chapterId];
    if (stored) return stored;
    return { chapterId, revision: 0, text: "", blocks: [] };
  },

  async saveChapter(chapterId: string, text: string, blocks?: ContentBlock[]): Promise<ChapterBody> {
    if (isTauriRuntime()) {
      return command<ChapterBody>("save_chapter", { chapterId, text, blocks: blocks ?? null });
    }
    const previous = memory.texts[chapterId] ?? { chapterId, revision: 0, text: "", blocks: [] };
    const sameText = previous.text === text;
    const sameBlocks =
      JSON.stringify(previous.blocks.map(blockContent)) ===
      JSON.stringify((blocks ?? previous.blocks).map(blockContent));
    const revision = sameText && sameBlocks ? previous.revision : previous.revision + 1;
    const body: ChapterBody = {
      chapterId,
      revision,
      text,
      blocks: blocks ?? previous.blocks,
    };
    memory.texts[chapterId] = body;
    const chapter = memory.chapters.find((item) => item.id === chapterId);
    if (chapter) chapter.currentRevision = revision;
    return body;
  },

  async renameProject(projectId: string, title: string): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("rename_project", { projectId, title });
    }
    const project = memory.projects.find((item) => item.id === projectId);
    if (!project) throw new Error("作品不存在");
    project.title = title;
    project.updatedAt = nowIso();
    return snapshot(projectId);
  },

  async deleteProject(projectId: string): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("delete_project", { projectId });
    }
    memory.books = memory.books.filter((book) => book.projectId !== projectId);
    memory.chapters = memory.chapters.filter((chapter) =>
      memory.books.some((book) => book.id === chapter.bookId),
    );
    memory.projects = memory.projects.filter((item) => item.id !== projectId);
    if (memory.activeProjectId === projectId) {
      memory.activeProjectId = memory.projects[0]?.id;
    }
    return snapshot();
  },

  async renameBook(projectId: string, bookId: string, title: string): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("rename_book", { projectId, bookId, title });
    }
    const book = memory.books.find((item) => item.id === bookId && item.projectId === projectId);
    if (!book) throw new Error("书不存在");
    book.title = title;
    return snapshot(projectId);
  },

  async deleteBook(projectId: string, bookId: string): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("delete_book", { projectId, bookId });
    }
    memory.chapters = memory.chapters.filter((chapter) => chapter.bookId !== bookId);
    memory.books = memory.books.filter((book) => book.id !== bookId);
    return snapshot(projectId);
  },

  async moveBook(projectId: string, bookId: string, delta: number): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("move_book", { projectId, bookId, delta });
    }
    memory.books = moveById(
      memory.books.filter((book) => book.projectId === projectId),
      bookId,
      delta,
    ).concat(memory.books.filter((book) => book.projectId !== projectId));
    return snapshot(projectId);
  },

  async renameChapter(
    projectId: string,
    chapterId: string,
    title: string,
  ): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("rename_chapter", { projectId, chapterId, title });
    }
    const chapter = memory.chapters.find((item) => item.id === chapterId);
    if (!chapter) throw new Error("章节不存在");
    chapter.title = title;
    return snapshot(projectId);
  },

  async deleteChapter(projectId: string, chapterId: string): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("delete_chapter", { projectId, chapterId });
    }
    memory.chapters = memory.chapters.filter((item) => item.id !== chapterId);
    delete memory.texts[chapterId];
    return snapshot(projectId);
  },

  async moveChapter(
    projectId: string,
    chapterId: string,
    delta: number,
  ): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("move_chapter", { projectId, chapterId, delta });
    }
    const chapter = memory.chapters.find((item) => item.id === chapterId);
    if (!chapter) throw new Error("章节不存在");
    const bookId = chapter.bookId;
    const siblings = moveById(
      memory.chapters.filter((item) => item.bookId === bookId),
      chapterId,
      delta,
    );
    memory.chapters = [
      ...memory.chapters.filter((item) => item.bookId !== bookId),
      ...siblings,
    ];
    return snapshot(projectId);
  },
};

function blockContent(block: ContentBlock): Pick<ContentBlock, "kind" | "text" | "position" | "markup"> {
  return { kind: block.kind, text: block.text, position: block.position, markup: block.markup };
}

function moveById<T extends { id: string; position: number }>(
  items: T[],
  id: string,
  delta: number,
): T[] {
  const ordered = items.slice().sort((a, b) => a.position - b.position);
  const index = ordered.findIndex((item) => item.id === id);
  if (index < 0) return ordered;
  const target = index + delta;
  if (target < 0 || target >= ordered.length) return ordered;
  const swapped = ordered.slice();
  const current = swapped[index];
  swapped[index] = swapped[target];
  swapped[target] = current;
  swapped.forEach((item, position) => {
    item.position = position + 1;
  });
  return swapped;
}
