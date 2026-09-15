import { describe, expect, it } from "vitest";
import { en } from "./catalogs/en";
import { zhCN } from "./catalogs/zh-CN";
import {
  catalogKeys,
  collatorLocale,
  formatDiffSummary,
  getLocale,
  interpolate,
  joinList,
  resetLocaleForTests,
  setLocale,
  t,
} from "./index";

describe("i18n catalogs", () => {
  it("keeps English keys aligned with Chinese", () => {
    expect(catalogKeys(en).sort()).toEqual(catalogKeys(zhCN).sort());
  });

  it("interpolates placeholders and leaves unknown ones intact", () => {
    expect(interpolate("Hello {name}", { name: "林默" })).toBe("Hello 林默");
    expect(interpolate("Hello {name}")).toBe("Hello {name}");
    expect(interpolate("Keep {missing}", { name: "x" })).toBe("Keep {missing}");
  });

  it("switches chrome copy with the active locale", () => {
    resetLocaleForTests("zh-CN");
    expect(t("chrome.continue")).toBe("续写");
    expect(t("library.bookCount", { count: 2 })).toBe("2 本书");
    expect(joinList(["林默", "雾儿"])).toBe("林默、雾儿");
    expect(formatDiffSummary(3, 1)).toBe("+3 字，-1 字");

    setLocale("en");
    expect(getLocale()).toBe("en");
    expect(collatorLocale()).toBe("en");
    expect(t("chrome.continue")).toBe("Continue");
    expect(t("library.bookCount", { count: 2 })).toBe("2 books");
    expect(joinList(["Lin Mo", "Fog"])).toBe("Lin Mo, Fog");
    expect(formatDiffSummary(3, 1)).toBe("+3 chars, -1 chars");
    expect(t("locale.hint")).toContain("writing protocol");
    expect(t("guide.bodyMissing", { count: 2 })).toContain("2");
    expect(t("tag.character")).toBe("Character");
  });

  it("persists the locale choice", () => {
    const store = new Map<string, string>();
    const memory = {
      getItem: (key: string) => store.get(key) ?? null,
      setItem: (key: string, value: string) => {
        store.set(key, value);
      },
      removeItem: (key: string) => {
        store.delete(key);
      },
      clear: () => store.clear(),
      key: (index: number) => [...store.keys()][index] ?? null,
      get length() {
        return store.size;
      },
    };
    Object.defineProperty(globalThis, "localStorage", { configurable: true, value: memory });

    setLocale("en");
    expect(store.get("moshu.locale")).toBe("en");
    resetLocaleForTests("zh-CN");
  });
});
