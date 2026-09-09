import { describe, expect, it } from "vitest";
import examples from "../../../packages/shared-types/examples.json";
import { Book, Chapter, Project, RevisionDiff, RevisionSummary, Scene, StoryEntry, Volume } from "./types";

describe("shared IPC examples", () => {
  it("match the frontend domain shapes", () => {
    const project = examples.project as Project;
    const book = examples.book as Book;
    const volume = examples.volume as Volume;
    const chapter = examples.chapter as Chapter;
    const scene = examples.scene as Scene;
    const entry = examples.storyEntry as StoryEntry;
    const revision = examples.revisionSummary as RevisionSummary;
    const diff = examples.revisionDiff as RevisionDiff;
    expect(project.title).toBe("夜航星图");
    expect(book.projectId).toBe(project.id);
    expect(volume.bookId).toBe(book.id);
    expect(chapter.volumeId).toBe(volume.id);
    expect(scene.chapterId).toBe(chapter.id);
    expect(entry.aliases).toEqual(["雾儿"]);
    expect(revision.revision).toBe(2);
    expect(diff.insertedChars).toBe(6);
  });
});
