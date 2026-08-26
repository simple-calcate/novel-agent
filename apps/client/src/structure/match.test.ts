import { describe, expect, it } from "vitest";
import { matchStoryEntries } from "./match";
import { StoryEntry } from "../types";

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
});
