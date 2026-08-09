import { describe, expect, it } from "vitest";
import { manifestSchema } from "./index";

describe("plugin sdk", () => {
  it("manifest schema is valid JSON schema", () => {
    expect(manifestSchema.type).toBe("object");
    expect(manifestSchema.required).toContain("id");
    expect(manifestSchema.required).toContain("operations");
  });
});
