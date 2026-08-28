import { describe, expect, it, beforeEach } from "vitest";
import { libraryApi, resetMemoryLibrary } from "./api";

describe("memory library", () => {
  beforeEach(() => {
    resetMemoryLibrary();
  });

  it("creates project, book and chapter then roundtrips text", async () => {
    const project = await libraryApi.createProject("夜航星图");
    const book = await libraryApi.createBook(project.id, "卷一");
    const extra = await libraryApi.createBook(project.id, "卷二");
    const chapter = await libraryApi.createChapter(project.id, extra.id, "第一章");

    const library = await libraryApi.loadLibrary(project.id);
    expect(library.books.map((item) => item.title)).toEqual(["卷一", "卷二"]);
    expect(library.chapters).toHaveLength(1);
    expect(book.position).toBe(1);

    await libraryApi.saveChapter(chapter.id, "雾港来客。");
    const loaded = await libraryApi.loadChapter(chapter.id);
    expect(loaded.text).toBe("雾港来客。");
    expect(loaded.revision).toBe(1);

    await libraryApi.renameBook(project.id, extra.id, "卷二改");
    await libraryApi.moveBook(project.id, extra.id, -1);
    const ordered = await libraryApi.loadLibrary(project.id);
    expect(ordered.books.map((item) => item.title)).toEqual(["卷二改", "卷一"]);

    await libraryApi.deleteChapter(project.id, chapter.id);
    await libraryApi.deleteBook(project.id, extra.id);
    const after = await libraryApi.loadLibrary(project.id);
    expect(after.chapters).toHaveLength(0);
    expect(after.books).toHaveLength(1);
  });

  it("groups chapters under a volume and ungroups on delete", async () => {
    const project = await libraryApi.createProject("夜航星图");
    const book = await libraryApi.createBook(project.id, "雾港纪事");
    const volume = await libraryApi.createVolume(project.id, book.id, "卷一");
    await libraryApi.createChapter(project.id, book.id, "第一章", volume.id);
    const library = await libraryApi.loadLibrary(project.id);
    expect(library.volumes?.map((item) => item.title)).toEqual(["卷一"]);
    expect(library.chapters[0].volumeId).toBe(volume.id);

    await libraryApi.renameVolume(project.id, volume.id, "上卷");
    await libraryApi.createVolume(project.id, book.id, "卷二");
    await libraryApi.moveVolume(project.id, volume.id, 1);
    const ordered = await libraryApi.loadLibrary(project.id);
    expect(ordered.volumes?.map((item) => item.title)).toEqual(["卷二", "上卷"]);

    await libraryApi.deleteVolume(project.id, volume.id);
    const after = await libraryApi.loadLibrary(project.id);
    expect(after.volumes).toHaveLength(1);
    expect(after.chapters[0].volumeId).toBeNull();
  });

  it("extracts canon candidates and accepts them", async () => {
    const project = await libraryApi.createProject("夜航星图");
    const book = await libraryApi.createBook(project.id, "卷一");
    const chapter = await libraryApi.createChapter(project.id, book.id, "第一章");
    await libraryApi.saveChapter(chapter.id, "林晚说道：「今夜雾很重。」走进雾港码头。");

    const created = await libraryApi.proposeCanon(chapter.id);
    expect(created.some((item) => item.entityName === "林晚")).toBe(true);
    const again = await libraryApi.proposeCanon(chapter.id);
    expect(again).toHaveLength(0);

    const candidates = await libraryApi.listCanon(project.id, "candidate");
    expect(candidates.length).toBeGreaterThan(0);
    await libraryApi.reviewCanonFact(candidates[0].factId, true);
    const accepted = await libraryApi.listCanon(project.id, "accepted");
    expect(accepted).toHaveLength(1);
    const leftover = await libraryApi.listCanon(project.id, "candidate");
    expect(leftover.every((item) => item.factId !== accepted[0].factId)).toBe(true);
  });

  it("stores designed story structure and lists it", async () => {
    const project = await libraryApi.createProject("夜航星图");
    await libraryApi.createStoryEntry(project.id, "character", "林晚", "雾港来的刀客");
    await libraryApi.createStoryEntry(project.id, "foreshadow", "雾中灯塔", "里面还有旧王玺");
    const entries = await libraryApi.listStoryEntries(project.id);
    expect(entries.map((item) => item.title)).toEqual(["林晚", "雾中灯塔"]);
    expect(entries[0].aliases).toEqual([]);
    await libraryApi.deleteStoryEntry(project.id, entries[0].id, entries[0].kind);
    const leftover = await libraryApi.listStoryEntries(project.id);
    expect(leftover).toHaveLength(1);
    expect(leftover[0].title).toBe("雾中灯塔");
    const withAlias = await libraryApi.createStoryEntry(project.id, "character", "沈雾、雾儿", "");
    expect(withAlias.title).toBe("沈雾");
    expect(withAlias.aliases).toEqual(["雾儿"]);
    await expect(
      libraryApi.createStoryEntry(project.id, "foreshadow", "雾中灯塔", "重复"),
    ).rejects.toThrow();
  });

  it("outlines scenes under a chapter without deleting text", async () => {
    const project = await libraryApi.createProject("夜航星图");
    const book = await libraryApi.createBook(project.id, "雾港纪事");
    const chapter = await libraryApi.createChapter(project.id, book.id, "第一章");
    await libraryApi.saveChapter(chapter.id, "雾港来客。");
    const scene = await libraryApi.createScene(project.id, chapter.id, "码头夜谈");
    const library = await libraryApi.loadLibrary(project.id);
    expect(library.scenes?.map((item) => item.title)).toEqual(["码头夜谈"]);
    await libraryApi.renameScene(project.id, scene.id, "码头雨夜");
    await libraryApi.createScene(project.id, chapter.id, "离开");
    await libraryApi.moveScene(project.id, scene.id, 1);
    const ordered = await libraryApi.loadLibrary(project.id);
    expect(ordered.scenes?.map((item) => item.title)).toEqual(["离开", "码头雨夜"]);
    await libraryApi.deleteScene(project.id, scene.id);
    const after = await libraryApi.loadLibrary(project.id);
    expect(after.scenes).toHaveLength(1);
    expect((await libraryApi.loadChapter(chapter.id)).text).toBe("雾港来客。");
  });

  it("records and disables writing preferences", async () => {
    const project = await libraryApi.createProject("夜航星图");
    const first = await libraryApi.recordGenerationFeedback(project.id, false, "AI 草稿");
    expect(first).toHaveLength(1);
    const again = await libraryApi.recordGenerationFeedback(project.id, false, "另一段");
    expect(again[0].status).toBe("confirmed");
    const disabled = await libraryApi.setPreferenceStatus(project.id, again[0].id, true);
    expect(disabled[0].status).toBe("disabled");
  });

  it("lists bundled plugins and counts names in the browser preview", async () => {
    const plugins = await libraryApi.listPlugins();
    expect(plugins.some((plugin) => plugin.id === "hello-names")).toBe(true);
    const result = await libraryApi.runPluginOperation("hello-names", "count-names", {
      selection: "林晚走进雾港，林晚没有回头",
      names: ["林晚", "雾儿"],
    });
    expect(result.output).toEqual({ counts: { 林晚: 2, 雾儿: 0 } });
  });

  it("reports empty outbox in the browser preview", async () => {
    expect(await libraryApi.pendingOutboxCount()).toBe(0);
    const flushed = await libraryApi.flushOutboxJournal();
    expect(flushed.written).toBe(0);
    expect(flushed.note).toContain("不是设备间同步");
  });
});
