import type { ContentBlock, StoryEntryKind } from "../types";
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

/** 示例预先设计的结构。思考里的 `@人物` 仍然不是入库；这些条是作者会自己加的那一类。 */
export const SAMPLE_STORY: Array<{ kind: StoryEntryKind; title: string; summary: string }> = [
  {
    kind: "character",
    title: "林默",
    summary: "雾港来客的主角。站在窗前看雾，手里攥着旧怀表。只写他所见。",
  },
  {
    kind: "setting",
    title: "雾港码头",
    summary: "石阶、铁索和潮声。雾先于潮声漫进港口，灯笼的光到不了这边。",
  },
  {
    kind: "foreshadow",
    title: "怀表来历",
    summary: "表盖内侧刻着两个字，笔画浅得像被潮气咬过。来历本章不解释。",
  },
];

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

  await ensureSampleStory(project.id);

  return { projectId: project.id, bookId: book.id, chapterId: chapter.id, created };
}

async function ensureSampleStory(projectId: string): Promise<void> {
  const existing = await libraryApi.listStoryEntries(projectId);
  for (const item of SAMPLE_STORY) {
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
