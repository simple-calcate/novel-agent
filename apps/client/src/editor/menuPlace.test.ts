import { describe, expect, it } from "vitest";
import { placeFloatingMenu } from "./menuPlace";

describe("placeFloatingMenu", () => {
  const viewport = { width: 1000, height: 600 };

  it("places the menu below the caret", () => {
    const placed = placeFloatingMenu({ top: 40, left: 80, bottom: 58 }, { height: 200, viewport });
    expect(placed.top).toBe(64);
    expect(placed.left).toBe(80);
  });

  it("flips above when the caret is near the bottom", () => {
    const placed = placeFloatingMenu({ top: 520, left: 80, bottom: 538 }, { height: 200, viewport });
    expect(placed.top).toBe(314);
  });

  it("clamps to the right edge", () => {
    const placed = placeFloatingMenu({ top: 40, left: 900, bottom: 58 }, { width: 240, viewport });
    expect(placed.left).toBe(752);
  });
});
