import { describe, it, expect } from "vitest";
import { isFreshLineState, nextMode } from "./ModeSwitch";

describe("isFreshLineState - Tab 生效状态判定", () => {
  it("空正文段行首：生效", () => {
    expect(
      isFreshLineState({ parentType: "paragraph", parentOffset: 0, textLength: 0 }),
    ).toBe(true);
  });

  it("空思考块行首：生效", () => {
    expect(
      isFreshLineState({ parentType: "thinkingBlock", parentOffset: 0, textLength: 0 }),
    ).toBe(true);
  });

  it("非行首（光标不在块起始）：不生效", () => {
    expect(
      isFreshLineState({ parentType: "paragraph", parentOffset: 2, textLength: 0 }),
    ).toBe(false);
  });

  it("非空行（已有内容）：不生效", () => {
    expect(
      isFreshLineState({ parentType: "paragraph", parentOffset: 0, textLength: 5 }),
    ).toBe(false);
  });

  it("其他块类型（标题/列表）：不生效", () => {
    expect(
      isFreshLineState({ parentType: "heading", parentOffset: 0, textLength: 0 }),
    ).toBe(false);
  });
});

describe("nextMode - 切换目标模式", () => {
  it("正文 -> 思考", () => {
    expect(nextMode("paragraph")).toBe("thinking");
  });

  it("思考 -> 正文", () => {
    expect(nextMode("thinkingBlock")).toBe("body");
  });

  it("不可切换类型返回 null", () => {
    expect(nextMode("heading")).toBeNull();
  });
});
