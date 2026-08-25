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
});
