/**
 * 客户端对宿主 IPC 的唯一入口。
 *
 * 桌面（Tauri）走命令层；浏览器预览走内存实现，便于无后端时开发 UI。
 * 新增作品/书/章能力时，先补这里再改界面。
 */
import { invoke } from "@tauri-apps/api/core";
import {
  Book,
  CanonProposal,
  Chapter,
  ChapterBody,
  CommandResult,
  ContentBlock,
  ContinuationPatch,
  ContextHint,
  FactStatus,
  LibrarySnapshot,
  ModelConfig,
  PluginSummary,
  PluginRunResult,
  OutboxFlushResult,
  PreferenceRule,
  Project,
  Scene,
  StoryEntry,
  StoryEntryKind,
  Volume,
} from "./types";
import { extractMentions } from "./canon/extract";
import { splitTitleAndAliases, matchStoryEntries } from "./structure/match";
// 必须静态导入。动态 import 在浏览器里第一次点「运行」会得到 undefined.call。
import { countNames } from "@novel-agent/plugin-sdk";

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
  canon: CanonProposal[];
  story: StoryEntry[];
  volumes: Volume[];
  scenes: Scene[];
  preferences: PreferenceRule[];
  modelConfig: ModelConfig | null;
}

const memory: MemoryState = {
  projects: [],
  books: [],
  chapters: [],
  texts: {},
  activeProjectId: undefined,
  canon: [],
  story: [],
  volumes: [],
  scenes: [],
  preferences: [],
  modelConfig: null,
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
    volumes: memory.volumes.filter((volume) =>
      memory.books.some((book) => book.id === volume.bookId && book.projectId === active),
    ),
    chapters: memory.chapters.filter((chapter) =>
      memory.books.some((book) => book.id === chapter.bookId && book.projectId === active),
    ),
    scenes: memory.scenes.filter((scene) =>
      memory.chapters.some(
        (chapter) =>
          chapter.id === scene.chapterId &&
          memory.books.some((book) => book.id === chapter.bookId && book.projectId === active),
      ),
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
  memory.canon = [];
  memory.story = [];
  memory.volumes = [];
  memory.scenes = [];
  memory.preferences = [];
  memory.modelConfig = null;
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

  async createChapter(
    projectId: string,
    bookId: string,
    title: string,
    volumeId?: string | null,
  ): Promise<Chapter> {
    if (isTauriRuntime()) {
      return command<Chapter>("create_chapter", {
        input: { projectId, bookId, title, position: 0, volumeId: volumeId ?? null },
      });
    }
    const siblings = memory.chapters.filter(
      (chapter) =>
        chapter.bookId === bookId && (chapter.volumeId ?? null) === (volumeId ?? null),
    );
    const chapter: Chapter = {
      id: newId(),
      bookId,
      volumeId: volumeId ?? null,
      title,
      position: siblings.reduce((max, item) => Math.max(max, item.position), 0) + 1,
      currentRevision: 0,
      status: "draft",
    };
    memory.chapters.push(chapter);
    memory.texts[chapter.id] = { chapterId: chapter.id, revision: 0, text: "", blocks: [] };
    return chapter;
  },

  async createVolume(projectId: string, bookId: string, title: string): Promise<Volume> {
    if (isTauriRuntime()) {
      return command<Volume>("create_volume", {
        input: { projectId, bookId, title, position: 0 },
      });
    }
    if (!memory.books.some((book) => book.id === bookId && book.projectId === projectId)) {
      throw new Error("书不存在");
    }
    const siblings = memory.volumes.filter((volume) => volume.bookId === bookId);
    const volume: Volume = {
      id: newId(),
      bookId,
      title,
      position: siblings.reduce((max, item) => Math.max(max, item.position), 0) + 1,
    };
    memory.volumes.push(volume);
    return volume;
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
    memory.volumes = memory.volumes.filter((volume) =>
      memory.books.some((book) => book.id === volume.bookId),
    );
    memory.chapters = memory.chapters.filter((chapter) =>
      memory.books.some((book) => book.id === chapter.bookId),
    );
    memory.projects = memory.projects.filter((item) => item.id !== projectId);
    memory.canon = memory.canon.filter((item) => item.projectId !== projectId);
    memory.story = memory.story.filter((item) => item.projectId !== projectId);
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
    memory.volumes = memory.volumes.filter((volume) => volume.bookId !== bookId);
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
    memory.scenes = memory.scenes.filter((item) => item.chapterId !== chapterId);
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
      memory.chapters.filter(
        (item) => item.bookId === bookId && (item.volumeId ?? null) === (chapter.volumeId ?? null),
      ),
      chapterId,
      delta,
    );
    memory.chapters = [
      ...memory.chapters.filter(
        (item) =>
          !(item.bookId === bookId && (item.volumeId ?? null) === (chapter.volumeId ?? null)),
      ),
      ...siblings,
    ];
    return snapshot(projectId);
  },

  async renameVolume(
    projectId: string,
    volumeId: string,
    title: string,
  ): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("rename_volume", { projectId, volumeId, title });
    }
    const volume = memory.volumes.find((item) => item.id === volumeId);
    if (!volume) throw new Error("卷不存在");
    volume.title = title;
    return snapshot(projectId);
  },

  async deleteVolume(projectId: string, volumeId: string): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("delete_volume", { projectId, volumeId });
    }
    memory.chapters.forEach((chapter) => {
      if (chapter.volumeId === volumeId) chapter.volumeId = null;
    });
    memory.volumes = memory.volumes.filter((item) => item.id !== volumeId);
    return snapshot(projectId);
  },

  async moveVolume(
    projectId: string,
    volumeId: string,
    delta: number,
  ): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("move_volume", { projectId, volumeId, delta });
    }
    const volume = memory.volumes.find((item) => item.id === volumeId);
    if (!volume) throw new Error("卷不存在");
    const bookId = volume.bookId;
    const siblings = moveById(
      memory.volumes.filter((item) => item.bookId === bookId),
      volumeId,
      delta,
    );
    memory.volumes = [
      ...memory.volumes.filter((item) => item.bookId !== bookId),
      ...siblings,
    ];
    return snapshot(projectId);
  },

  async proposeCanon(chapterId: string): Promise<CanonProposal[]> {
    if (isTauriRuntime()) {
      return command<CanonProposal[]>("propose_canon", { chapterId });
    }
    const chapter = memory.chapters.find((item) => item.id === chapterId);
    const book = memory.books.find((item) => item.id === chapter?.bookId);
    if (!chapter || !book) throw new Error("章节不存在");
    const text = memory.texts[chapterId]?.text ?? "";
    const created: CanonProposal[] = [];
    for (const mention of extractMentions(text)) {
      const duplicate = memory.canon.some(
        (item) =>
          item.projectId === book.projectId &&
          item.entityName === mention.entityName &&
          item.entityKind === mention.entityKind &&
          item.predicate === mention.predicate &&
          item.chapterId === chapterId,
      );
      if (duplicate) continue;
      created.push({
        factId: newId(),
        entityId: newId(),
        projectId: book.projectId,
        chapterId,
        entityName: mention.entityName,
        entityKind: mention.entityKind,
        predicate: mention.predicate,
        object: mention.object,
        quote: mention.quote,
        status: "candidate",
        confidence: mention.confidence,
      });
    }
    memory.canon.push(...created);
    return created;
  },

  async listCanon(projectId: string, status?: FactStatus): Promise<CanonProposal[]> {
    if (isTauriRuntime()) {
      return command<CanonProposal[]>("list_canon", { projectId, status: status ?? null });
    }
    return memory.canon.filter(
      (item) => item.projectId === projectId && (status ? item.status === status : true),
    );
  },

  async reviewCanonFact(factId: string, accept: boolean): Promise<CanonProposal> {
    if (isTauriRuntime()) {
      return command<CanonProposal>("review_canon_fact", { factId, accept });
    }
    const fact = memory.canon.find((item) => item.factId === factId);
    if (!fact) throw new Error("正史条目不存在");
    fact.status = accept ? "accepted" : "rejected";
    return fact;
  },

  async createStoryEntry(
    projectId: string,
    kind: StoryEntryKind,
    title: string,
    summary = "",
  ): Promise<StoryEntry> {
    if (isTauriRuntime()) {
      return command<StoryEntry>("create_story_entry", { projectId, kind, title, summary });
    }
    const parsed = splitTitleAndAliases(title);
    if (!parsed.title) throw new Error("请填写名称");
    if (
      memory.story.some(
        (item) => item.projectId === projectId && item.kind === kind && item.title === parsed.title,
      )
    ) {
      throw new Error("该结构已存在");
    }
    const entry: StoryEntry = {
      id: newId(),
      projectId,
      kind,
      title: parsed.title,
      summary,
      aliases: parsed.aliases,
    };
    memory.story.push(entry);
    return entry;
  },

  async listStoryEntries(projectId: string): Promise<StoryEntry[]> {
    if (isTauriRuntime()) {
      return command<StoryEntry[]>("list_story_entries", { projectId });
    }
    return memory.story
      .filter((item) => item.projectId === projectId)
      .slice()
      .sort((left, right) => kindOrder(left.kind) - kindOrder(right.kind) || left.title.localeCompare(right.title, "zh"));
  },

  async deleteStoryEntry(projectId: string, id: string, kind: StoryEntryKind): Promise<void> {
    if (isTauriRuntime()) {
      await command("delete_story_entry", { projectId, id, kind });
      return;
    }
    memory.story = memory.story.filter((item) => item.id !== id);
  },

  async createScene(
    projectId: string,
    chapterId: string,
    title: string,
    povEntryId?: string | null,
  ): Promise<Scene> {
    if (isTauriRuntime()) {
      return command<Scene>("create_scene", {
        input: { projectId, chapterId, title, position: 0, povEntryId: povEntryId ?? null },
      });
    }
    const chapter = memory.chapters.find((item) => item.id === chapterId);
    if (!chapter) throw new Error("章节不存在");
    const siblings = memory.scenes.filter((item) => item.chapterId === chapterId);
    const scene: Scene = {
      id: newId(),
      chapterId,
      title,
      position: siblings.reduce((max, item) => Math.max(max, item.position), 0) + 1,
      povEntryId: povEntryId || null,
    };
    memory.scenes.push(scene);
    return scene;
  },

  async renameScene(projectId: string, sceneId: string, title: string): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("rename_scene", { projectId, sceneId, title });
    }
    const scene = memory.scenes.find((item) => item.id === sceneId);
    if (!scene) throw new Error("场次不存在");
    scene.title = title;
    return snapshot(projectId);
  },

  async setScenePov(
    projectId: string,
    sceneId: string,
    povEntryId?: string | null,
  ): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("set_scene_pov", {
        projectId,
        sceneId,
        povEntryId: povEntryId ?? null,
      });
    }
    const scene = memory.scenes.find((item) => item.id === sceneId);
    if (!scene) throw new Error("场次不存在");
    scene.povEntryId = povEntryId || null;
    return snapshot(projectId);
  },

  async deleteScene(projectId: string, sceneId: string): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("delete_scene", { projectId, sceneId });
    }
    memory.scenes = memory.scenes.filter((item) => item.id !== sceneId);
    return snapshot(projectId);
  },

  async moveScene(projectId: string, sceneId: string, delta: number): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return command<LibrarySnapshot>("move_scene", { projectId, sceneId, delta });
    }
    const scene = memory.scenes.find((item) => item.id === sceneId);
    if (!scene) throw new Error("场次不存在");
    const chapterId = scene.chapterId;
    const siblings = moveById(
      memory.scenes.filter((item) => item.chapterId === chapterId),
      sceneId,
      delta,
    );
    memory.scenes = [
      ...memory.scenes.filter((item) => item.chapterId !== chapterId),
      ...siblings,
    ];
    return snapshot(projectId);
  },

  async loadModelConfig(): Promise<ModelConfig | null> {
    if (isTauriRuntime()) {
      return invoke<ModelConfig | null>("load_model_config");
    }
    return memory.modelConfig;
  },

  async saveModelConfig(config: ModelConfig): Promise<void> {
    if (isTauriRuntime()) {
      await invoke("save_model_config", { config });
      return;
    }
    memory.modelConfig = {
      ...config,
      apiKey: "",
      apiKeySet: Boolean(config.apiKey) || Boolean(config.apiKeySet),
    };
  },

  async contextHints(input: {
    projectId: string;
    chapterId: string;
    revision: number;
    nearbyText: string;
    lookbackText?: string;
    generation: number;
  }): Promise<ContextHint[]> {
    if (isTauriRuntime()) {
      return command<ContextHint[]>("context_hints", { input });
    }
    return matchStoryEntries(
      input.nearbyText,
      input.lookbackText ?? "",
      memory.story.filter((item) => item.projectId === input.projectId),
      input.revision,
    );
  },

  async generateContinuation(input: {
    chapterId: string;
    revision: number;
    prompt: string;
    contextText: string;
    config?: ModelConfig | null;
  }): Promise<ContinuationPatch> {
    if (isTauriRuntime()) {
      return invoke<ContinuationPatch>("generate_continuation", {
        chapterId: input.chapterId,
        revision: input.revision,
        prompt: input.prompt,
        contextText: input.contextText,
        config: input.config ? { ...input.config, apiKey: "" } : undefined,
      });
    }
    throw new Error("浏览器预览不能调用模型，请用桌面应用续写");
  },

  async recordGenerationFeedback(
    projectId: string,
    accepted: boolean,
    aiText: string,
    humanText = "",
    contextExcerpt = "",
  ): Promise<PreferenceRule[]> {
    if (isTauriRuntime()) {
      return command<PreferenceRule[]>("record_generation_feedback", {
        projectId,
        accepted,
        aiText,
        humanText,
        contextExcerpt,
      });
    }
    if (accepted) return memory.preferences.filter((item) => belongsToProject(item, projectId));
    const existing = memory.preferences.find(
      (item) => belongsToProject(item, projectId) && item.status !== "disabled",
    );
    if (existing) {
      existing.status = "confirmed";
      existing.updatedAt = nowIso();
      return memory.preferences.filter((item) => belongsToProject(item, projectId));
    }
    memory.preferences.push({
      id: newId(),
      scope: { projectId },
      rule: "尊重作者明确拒绝",
      status: "candidate",
      createdAt: nowIso(),
      updatedAt: nowIso(),
    });
    return memory.preferences.filter((item) => belongsToProject(item, projectId));
  },

  async listPreferences(projectId: string): Promise<PreferenceRule[]> {
    if (isTauriRuntime()) {
      return command<PreferenceRule[]>("list_preferences", { projectId });
    }
    return memory.preferences.filter((item) => belongsToProject(item, projectId));
  },

  async setPreferenceStatus(
    projectId: string,
    ruleId: string,
    disabled: boolean,
  ): Promise<PreferenceRule[]> {
    if (isTauriRuntime()) {
      return command<PreferenceRule[]>("set_preference_status", { projectId, ruleId, disabled });
    }
    const rule = memory.preferences.find((item) => item.id === ruleId);
    if (rule) {
      rule.status = disabled ? "disabled" : "confirmed";
      rule.updatedAt = nowIso();
    }
    return memory.preferences.filter((item) => belongsToProject(item, projectId));
  },

  async listPlugins(): Promise<PluginSummary[]> {
    if (isTauriRuntime()) {
      return command<PluginSummary[]>("list_plugins");
    }
    return [
      {
        id: "continuity-checker",
        name: "连续性检查",
        version: "0.1.0",
        runtime: "builtin",
        operations: ["check-chapter"],
      },
      {
        id: "hello-names",
        name: "人名点名",
        version: "0.1.0",
        runtime: "wasm",
        operations: ["count-names"],
      },
      {
        id: "summary-extractor",
        name: "章节摘要与实体抽取",
        version: "0.1.0",
        runtime: "builtin",
        operations: ["extract-story-delta"],
      },
      {
        id: "continuation-writer",
        name: "智能续写",
        version: "0.1.0",
        runtime: "builtin",
        operations: ["continue-scene"],
      },
    ];
  },

  async runPluginOperation(
    pluginId: string,
    operation: string,
    input: unknown,
  ): Promise<PluginRunResult> {
    if (isTauriRuntime()) {
      return command<PluginRunResult>("run_plugin_operation", {
        input: { pluginId, operation, input },
      });
    }
    if (pluginId === "hello-names" && operation === "count-names") {
      const payload = (input ?? {}) as { selection?: string; names?: string[] };
      const result = countNames(payload.selection ?? "", payload.names ?? []);
      return { output: result.output, logs: result.logs ?? ["hello-names"] };
    }
    return {
      output: {
        operation,
        message: "这是内置占位回执，还没有真正执行。浏览器预览只对人名点名走 SDK。",
      },
      logs: ["browser-preview"],
    };
  },

  async pendingOutboxCount(): Promise<number> {
    if (isTauriRuntime()) {
      return command<number>("pending_outbox_count");
    }
    return 0;
  },

  async flushOutboxJournal(): Promise<OutboxFlushResult> {
    if (isTauriRuntime()) {
      return command<OutboxFlushResult>("flush_outbox_journal");
    }
    return {
      written: 0,
      path: "",
      note: "浏览器预览没有 outbox，桌面才会把变更写成 JSONL。这不是设备间同步。",
    };
  },
};

function belongsToProject(rule: PreferenceRule, projectId: string): boolean {
  if (typeof rule.scope === "string") return true;
  return !rule.scope.projectId || rule.scope.projectId === projectId;
}

function kindOrder(kind: StoryEntryKind): number {
  if (kind === "character") return 0;
  if (kind === "setting") return 1;
  return 2;
}

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
