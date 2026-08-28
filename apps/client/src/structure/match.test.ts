import { describe, expect, it } from "vitest";
import { matchStoryEntries } from "./match";
import { StoryEntry } from "../types";
import cases from "../../../../packages/match-fixtures/cases.json";

const linWan: StoryEntry = {
  id: "1",
  projectId: "p",
  kind: "character",
  title: "林晚",
  summary: "雾港来的刀客",
  aliases: ["雾儿"],
};

const lighthouse: StoryEntry = {
  id: "2",
  projectId: "p",
  kind: "foreshadow",
  title: "雾中灯塔",
  summary: "里面还有旧王玺",
  aliases: [],
};

describe("structure matching", () => {
  it("ranks the full name above a place name in the same paragraph", () => {
    const fog: StoryEntry = {
      id: "3",
      projectId: "p",
      kind: "setting",
      title: "雾港",
      summary: "终年被海雾罩住",
      aliases: [],
    };
    const hints = matchStoryEntries("林晚走进雾港", "", [linWan, lighthouse, fog], 1);
    expect(hints.map((hint) => hint.title)).toEqual(["林晚", "雾港"]);
  });

  it("matches alias, keyword, title core and lookback", () => {
    expect(matchStoryEntries("雾儿没有回头", "", [linWan], 1)[0]?.title).toBe("林晚");
    expect(matchStoryEntries("那个刀客转过身", "", [linWan], 1)[0]?.matchReason).toContain("刀客");
    expect(matchStoryEntries("那座灯塔夜里忽然亮了", "", [lighthouse], 1)[0]?.title).toBe(
      "雾中灯塔",
    );
    expect(matchStoryEntries("旧王玺还在匣中", "", [lighthouse], 1)[0]?.matchReason).toContain(
      "旧王玺",
    );
    expect(matchStoryEntries("她没有回头", "林晚走进雾港", [linWan], 1)[0]?.matchReason).toContain(
      "上一段",
    );
  });

  it("hides unrelated paragraphs", () => {
    expect(matchStoryEntries("夜晚的海面", "", [linWan, lighthouse], 1)).toEqual([]);
  });

  it("retrieves an entry by inverted lexical search", () => {
    const keeper: StoryEntry = {
      id: "4",
      projectId: "p",
      kind: "character",
      title: "灯塔守夜人",
      summary: "负责在雾季敲钟",
      aliases: [],
    };
    const hints = matchStoryEntries("雾季快到了", "", [keeper], 1);
    expect(hints).toHaveLength(1);
    expect(hints[0]?.title).toBe("灯塔守夜人");
    expect(hints[0]?.matchReason).toContain("检索到");
    expect(hints[0]?.matchReason).toContain("雾季");
  });

  it("shares ranking cases with the rust matcher", () => {
    const fog: StoryEntry = {
      id: "3",
      projectId: "p",
      kind: "setting",
      title: "雾港",
      summary: "终年被海雾罩住",
      aliases: [],
    };
    for (const item of cases) {
      const extras = ((item as { extraEntries?: StoryEntry[] }).extraEntries ?? []).map(
        (entry) => ({
          ...entry,
          projectId: entry.projectId ?? "p",
          aliases: entry.aliases ?? [],
        }),
      );
      const titles = matchStoryEntries(
        item.current,
        item.lookback,
        [linWan, lighthouse, fog, ...extras],
        1,
      ).map((hint) => hint.title);
      expect(titles, item.id).toEqual(item.expectedTitles);
    }
  });
});
