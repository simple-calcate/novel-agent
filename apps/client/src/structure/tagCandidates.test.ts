import { describe, expect, it } from "vitest";
import { StoryEntry } from "../types";
import { filterTagKindLabels, listTagCandidates } from "./tagCandidates";

const linMo: StoryEntry = {
  id: "1",
  projectId: "p",
  kind: "character",
  title: "林默",
  summary: "雾港来客的主角",
  aliases: ["雾儿"],
};

const dock: StoryEntry = {
  id: "2",
  projectId: "p",
  kind: "setting",
  title: "雾港码头",
  summary: "石阶、铁索和潮声",
  aliases: [],
};

const watch: StoryEntry = {
  id: "3",
  projectId: "p",
  kind: "foreshadow",
  title: "怀表来历",
  summary: "表盖内侧两个字",
  aliases: [],
};

describe("tag candidates", () => {
  it("filters @ kind labels by the query after @", () => {
    expect(filterTagKindLabels("人")).toEqual(["人物"]);
    expect(filterTagKindLabels("")).toEqual(["人物", "伏笔", "地点", "道具", "势力", "规则"]);
  });

  it("only offers characters for @人物, including alias hits", () => {
    const names = listTagCandidates([linMo, dock, watch], "人物", "雾");
    expect(names.map((item) => item.name)).toEqual(["林默"]);
    expect(names[0]?.matchedAlias).toBe("雾儿");
  });

  it("offers foreshadow titles for @伏笔 and settings for @地点", () => {
    expect(listTagCandidates([linMo, dock, watch], "伏笔", "").map((item) => item.name)).toEqual([
      "怀表来历",
    ]);
    expect(listTagCandidates([linMo, dock, watch], "地点", "").map((item) => item.name)).toEqual([
      "雾港码头",
    ]);
  });

  it("ranks names that already appear near the caret first", () => {
    const extra: StoryEntry = {
      id: "4",
      projectId: "p",
      kind: "character",
      title: "阿潮",
      summary: "",
      aliases: [],
    };
    const names = listTagCandidates([extra, linMo], "人物", "", "林默站在窗前");
    expect(names.map((item) => item.name)).toEqual(["林默", "阿潮"]);
  });
});
