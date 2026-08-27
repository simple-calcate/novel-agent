import { describe, expect, it } from "vitest";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { helloNames } from "@novel-agent/plugin-sdk";
import { compileGuest } from "./compile";
import { inspectGuestWasm } from "./inspect";
import { attachWasm, packPlugin } from "./pack";
import { previewGuest } from "./preview";

const pkgRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const helloGuest = path.join(pkgRoot, "examples/hello-names.ts");

describe("plugin compile", () => {
  it("packs wasm into a manifest", () => {
    const wasm = new Uint8Array([0x00, 0x61, 0x73, 0x6d]);
    const packed = JSON.parse(packPlugin(helloNames, wasm)) as { id: string; wasmBase64: string };
    expect(packed.id).toBe("hello-names");
    expect(packed.wasmBase64).toBe(Buffer.from(wasm).toString("base64"));
    expect(attachWasm(helloNames, wasm).wasmBase64).toBe(packed.wasmBase64);
  });

  it("compiles hello-names without imports and counts names", async () => {
    const wasm = await compileGuest(helloGuest);
    const info = inspectGuestWasm(wasm);
    expect(info.imports).toEqual([]);
    expect(info.exports.some((item) => item.name === "memory" && item.kind === "memory")).toBe(true);
    expect(info.exports.some((item) => item.name === "plugin_execute" && item.kind === "func")).toBe(
      true,
    );

    const result = previewGuest(wasm, "count-names", {
      selection: "林晚走进雾港，林晚没有回头",
      names: ["林晚", "雾儿"],
    });
    expect(result.output).toEqual({ counts: { 林晚: 2, 雾儿: 0 } });
    expect(result.logs).toEqual(["hello-names"]);
  }, 30_000);

  it("returns an error payload for unknown operations", async () => {
    const wasm = await compileGuest(helloGuest);
    const result = previewGuest(wasm, "missing", {});
    expect(result.output).toEqual({ error: "unknown operation" });
  }, 30_000);
});
