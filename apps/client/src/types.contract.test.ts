import { describe, expect, it } from "vitest";
import examples from "../../../../packages/shared-types/examples.json";
import { Book, Chapter, Project, Scene, StoryEntry, Volume } from "./types";

describe("shared IPC examples", () => {
  it("match the frontend domain shapes", () => {
    const project: Project = examples.project;
    const book: Book = examples.book;
    const volume: Volume = examples.volume;
    const chapter: Chapter = examples.chapter;
    const scene: Scene = examples.scene;
    const entry: StoryEntry = examples.storyEntry;
    expect(project.title).toBe("夜航星图");
    expect(book.projectId).toBe(project.id);
    expect(volume.bookId).toBe(book.id);
    expect(chapter.volumeId).toBe(volume.id);
    expect(scene.chapterId).toBe(chapter.id);
    expect(entry.aliases).toEqual(["雾儿"]);
  });
});
