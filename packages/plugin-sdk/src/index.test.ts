import { describe, expect, it } from "vitest";
import { definePlugin, PluginManifestError, manifestSchema, toManifestJson } from "./index";
import { countNames, helloNames } from "./examples/hello-names";

describe("plugin sdk", () => {
  it("manifest schema is valid JSON schema", () => {
    expect(manifestSchema.type).toBe("object");
    expect(manifestSchema.required).toContain("id");
    expect(manifestSchema.required).toContain("operations");
  });

  it("definePlugin fills defaults and serializes", () => {
    const json = JSON.parse(toManifestJson(helloNames));
    expect(json.id).toBe("hello-names");
    expect(json.operations[0].name).toBe("count-names");
    expect(json.requestedCapabilities).toEqual([{ kind: "readSelection" }, { kind: "log" }]);
  });

  it("rejects invalid ids", () => {
    expect(() =>
      definePlugin({
        ...helloNames,
        id: "Hello",
      }),
    ).toThrow(PluginManifestError);
  });

  it("counts names in a selection", () => {
    const result = countNames("林晚走进雾港，林晚没有回头", ["林晚", "雾儿"]);
    expect(result.output).toEqual({ counts: { 林晚: 2, 雾儿: 0 } });
  });
});
