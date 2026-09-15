import type { ContentBlock, StoryEntryKind } from "../types";
import { libraryApi } from "../api";
import raw from "./examples/fog-harbor.json";

export interface SampleStoryEntry {
  kind: StoryEntryKind;
  title: string;
  summary: string;
}

export interface SampleChapterFile {
  projectTitle: string;
  bookTitle: string;
  bookSynopsis: string;
  chapterTitle: string;
  story: SampleStoryEntry[];
  blocks: ContentBlock[];
}

/** 打包进客户端的示例。正文和结构都在 JSON 里，这里只负责装进作品库。 */
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

/** 把《雾港来客》装进作品库。已有同名章节则只打开，不覆盖作者改过的字，也不把删掉的结构再种回来。 */
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
  let chapterCreated = false;
  if (!chapter) {
    chapter = await libraryApi.createChapter(project.id, book.id, sample.chapterTitle);
    chapterCreated = true;
  }

  const body = await libraryApi.loadChapter(chapter.id);
  let wroteBody = false;
  if (body.blocks.length === 0 && body.text.trim() === "") {
    await libraryApi.saveChapter(chapter.id, sampleBodyText(sample.blocks), sample.blocks);
    wroteBody = true;
  }

  // 结构只在新建示例章时写入。作者删掉林默后再点打开，不应复活。
  if (chapterCreated) {
    await ensureSampleStory(project.id, sample.story);
  }

  return {
    projectId: project.id,
    bookId: book.id,
    chapterId: chapter.id,
    created: chapterCreated || wroteBody,
  };
}

async function ensureSampleStory(projectId: string, story: SampleStoryEntry[]): Promise<void> {
  const existing = await libraryApi.listStoryEntries(projectId);
  for (const item of story) {
    if (existing.some((entry) => entry.kind === item.kind && entry.title === item.title)) {
      continue;
    }
    try {
      const created = await libraryApi.createStoryEntry(
        projectId,
        item.kind,
        item.title,
        item.summary,
      );
      existing.push(created);
    } catch {
      // 已有同名条目（作者改过或并发）就跳过，不覆盖。
    }
  }
}
