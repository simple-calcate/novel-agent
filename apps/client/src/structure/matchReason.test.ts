import { describe, expect, it } from "vitest";
import { formatMatchReason, parseMatchReason, encodeMatchReason } from "./matchReason";
import { resetLocaleForTests, setLocale } from "../i18n";

describe("match reason codes", () => {
  it("encodes and parses code:term, including colons in the term", () => {
    expect(encodeMatchReason("keyword", "刀客")).toBe("keyword:刀客");
    expect(parseMatchReason("title:雾港:码头")).toEqual({ code: "title", term: "雾港:码头" });
    expect(parseMatchReason("出现名称「林晚」")).toBeNull();
    expect(parseMatchReason("title:")).toBeNull();
  });

  it("formats with the active UI locale", () => {
    resetLocaleForTests("zh-CN");
    expect(formatMatchReason("alias:雾儿")).toBe("出现别名「雾儿」");
    expect(formatMatchReason("lookback:林晚")).toBe("上一段出现「林晚」");
    expect(formatMatchReason("retrieve:雾季")).toBe("检索到「雾季」");

    setLocale("en");
    expect(formatMatchReason("alias:雾儿")).toBe("Alias “雾儿” appears");
    expect(formatMatchReason("lookback:林晚")).toBe("Previous paragraph had “林晚”");
    expect(formatMatchReason("retrieve:雾季")).toBe("Retrieved “雾季”");
    expect(formatMatchReason("出现名称「林晚」")).toBe("出现名称「林晚」");
    resetLocaleForTests("zh-CN");
  });
});
