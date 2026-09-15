import { describe, expect, it } from "vitest";
import { libraryApi, resetMemoryLibrary } from "../api";
import {
  buildTrainingExamples,
  filterExamples,
  serializeExamples,
  WRITING_PROTOCOL_SYSTEM,
} from "./protocol";
import { installSampleChapter, sampleBodyText, sampleChapter } from "./sampleChapter";

describe("fog-harbor sample chapter", () => {
  it("keeps story entries in the sample json, not in installer code", () => {
    expect(sampleChapter.story.map((entry) => `${entry.kind}:${entry.title}`)).toEqual([
      "character:林默",
      "setting:雾港码头",
      "foreshadow:怀表来历",
    ]);
  });

  it("exports three complete beats from chapter start", () => {
    const examples = buildTrainingExamples(sampleChapter.blocks, true, sampleChapter.chapterTitle);
    expect(examples).toHaveLength(3);
    expect(examples.every((example) => example.quality === "gold")).toBe(true);
    expect(filterExamples(examples, "usable")).toHaveLength(3);

    expect(examples[0].context).toBe("");
    expect(examples[0].instruction).toBe(`写下《${sampleChapter.chapterTitle}》的开篇。`);
    expect(examples[0].thinking).toContain("意图：港口第一眼就要冷");
    expect(examples[0].thinking).toContain("@人物：林默");
    expect(examples[0].thinking).not.toContain("[@人物：林默]");
    expect(examples[0].content).toContain("雾先于潮声漫进港口。");

    expect(examples[1].instruction).toBe("续写下一段。");
    expect(examples[1].context).toContain("【思考】意图：港口第一眼就要冷");
    expect(examples[1].context).toContain("雾先于潮声漫进港口。");
    expect(examples[1].context).not.toContain("表盖掀开");
    expect(examples[1].thinking).toContain("@伏笔：怀表来历");
    expect(examples[1].content).toContain("他把表盖掀开一条缝。");

    expect(examples[2].context).toContain("他把表盖掀开一条缝。");
    expect(examples[2].context).toContain("【思考】意图：让表盖内侧的两个字入镜");
    expect(examples[2].content).toContain("灯笼从帆布里掏出来");
  });

  it("serializes sharegpt without a dummy continue prompt", () => {
    const examples = filterExamples(
      buildTrainingExamples(sampleChapter.blocks, true, sampleChapter.chapterTitle),
      "usable",
    );
    const sharegpt = serializeExamples(examples, "sharegpt");
    expect(sharegpt).toContain(WRITING_PROTOCOL_SYSTEM);
    expect(sharegpt).toContain(`写下《${sampleChapter.chapterTitle}》的开篇。`);
    expect(sharegpt).toContain("续写下一段。");
    expect(sharegpt).not.toContain("继续写作");
  });

  it("installs into an empty library and does not overwrite edits", async () => {
    resetMemoryLibrary();
    const first = await installSampleChapter();
    const loaded = await libraryApi.loadChapter(first.chapterId);
    expect(loaded.blocks).toHaveLength(sampleChapter.blocks.length);
    expect(loaded.text).toBe(sampleBodyText());
    expect(first.created).toBe(true);
    const story = await libraryApi.listStoryEntries(first.projectId);
    expect(story.map((entry) => `${entry.kind}:${entry.title}`)).toEqual([
      "character:林默",
      "setting:雾港码头",
      "foreshadow:怀表来历",
    ]);

    await libraryApi.saveChapter(first.chapterId, "作者改过的正文。", [
      {
        id: "99999999-9999-4999-8999-999999999999",
        kind: "body",
        text: "作者改过的正文。",
        position: 0,
        markup: [],
      },
    ]);

    const second = await installSampleChapter();
    expect(second.chapterId).toBe(first.chapterId);
    expect(second.created).toBe(false);
    const again = await libraryApi.loadChapter(second.chapterId);
    expect(again.text).toBe("作者改过的正文。");
    expect(await libraryApi.listStoryEntries(second.projectId)).toHaveLength(3);
  });

  it("does not resurrect a story entry the author deleted", async () => {
    resetMemoryLibrary();
    const first = await installSampleChapter();
    const story = await libraryApi.listStoryEntries(first.projectId);
    const linMo = story.find((entry) => entry.title === "林默");
    expect(linMo).toBeDefined();
    await libraryApi.deleteStoryEntry(first.projectId, linMo!.id, linMo!.kind);

    const second = await installSampleChapter();
    expect(second.chapterId).toBe(first.chapterId);
    expect(second.created).toBe(false);
    const leftover = await libraryApi.listStoryEntries(second.projectId);
    expect(leftover.map((entry) => entry.title)).toEqual(["雾港码头", "怀表来历"]);
  });

  it("does not restore deleted story when refilling an emptied sample chapter", async () => {
    resetMemoryLibrary();
    const first = await installSampleChapter();
    const story = await libraryApi.listStoryEntries(first.projectId);
    const linMo = story.find((entry) => entry.title === "林默");
    await libraryApi.deleteStoryEntry(first.projectId, linMo!.id, linMo!.kind);
    await libraryApi.saveChapter(first.chapterId, "", []);

    const second = await installSampleChapter();
    expect(second.chapterId).toBe(first.chapterId);
    expect(second.created).toBe(true);
    expect((await libraryApi.loadChapter(second.chapterId)).text).toBe(sampleBodyText());
    const leftover = await libraryApi.listStoryEntries(second.projectId);
    expect(leftover.map((entry) => entry.title)).not.toContain("林默");
  });

  it("reseeds story when the sample chapter itself was deleted", async () => {
    resetMemoryLibrary();
    const first = await installSampleChapter();
    await libraryApi.deleteChapter(first.projectId, first.chapterId);
    const leftover = await libraryApi.listStoryEntries(first.projectId);
    const linMo = leftover.find((entry) => entry.title === "林默");
    expect(linMo).toBeDefined();
    await libraryApi.deleteStoryEntry(first.projectId, linMo!.id, linMo!.kind);

    const second = await installSampleChapter();
    expect(second.chapterId).not.toBe(first.chapterId);
    expect(second.created).toBe(true);
    const story = await libraryApi.listStoryEntries(second.projectId);
    expect(story.map((entry) => `${entry.kind}:${entry.title}`)).toEqual([
      "character:林默",
      "setting:雾港码头",
      "foreshadow:怀表来历",
    ]);
  });
});
