import type { ContentBlock } from "../types";
import { libraryApi } from "../api";
import raw from "./examples/fog-harbor.json";

export interface SampleChapterFile {
  projectTitle: string;
  bookTitle: string;
  bookSynopsis: string;
  chapterTitle: string;
  blocks: ContentBlock[];
}

export const sampleChapter = raw as SampleChapterFile;

export function sampleBodyText(blocks: ContentBlock[] = sampleChapter.blocks): string {
  return blocks
    .filter((block) => block.kind === "body")
    .map((block) => block.text)
    .join("\n");
}

export interface InstalledSample {
  projectId: string;
  bookId: string;
  chapterId: string;
  created: boolean;
}

/** 把《雾港来客》装进作品库。已有同名章节则只打开，不覆盖作者改过的字。 */
export async function installSampleChapter(): Promise<InstalledSample> {
  const sample = sampleChapter;
  let snapshot = await libraryApi.loadLibrary();
  let project = snapshot.projects.find((item) => item.title === sample.projectTitle);
  if (!project) {
    project = await libraryApi.createProject(sample.projectTitle);
  }
  snapshot = await libraryApi.setActiveProject(project.id);

  let book = snapshot.books.find((item) => item.title === sample.bookTitle);
  if (!book) {
    book = await libraryApi.createBook(project.id, sample.bookTitle, sample.bookSynopsis);
    snapshot = await libraryApi.loadLibrary(project.id);
  }

  let chapter = snapshot.chapters.find(
    (item) => item.bookId === book.id && item.title === sample.chapterTitle,
  );
  let created = false;
  if (!chapter) {
    chapter = await libraryApi.createChapter(project.id, book.id, sample.chapterTitle);
    created = true;
  }

  const body = await libraryApi.loadChapter(chapter.id);
  if (body.blocks.length === 0 && body.text.trim() === "") {
    await libraryApi.saveChapter(chapter.id, sampleBodyText(sample.blocks), sample.blocks);
    created = true;
  }

  return { projectId: project.id, bookId: book.id, chapterId: chapter.id, created };
}
