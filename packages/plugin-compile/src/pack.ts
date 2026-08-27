import { definePlugin, PluginManifest, toManifestJson } from "@novel-agent/plugin-sdk";

export function encodeWasmBase64(wasm: Uint8Array): string {
  return Buffer.from(wasm).toString("base64");
}

export function attachWasm(manifest: PluginManifest, wasm: Uint8Array): PluginManifest {
  return definePlugin({
    ...manifest,
    wasmBase64: encodeWasmBase64(wasm),
  });
}

export function packPlugin(manifest: PluginManifest, wasm: Uint8Array): string {
  return toManifestJson(attachWasm(manifest, wasm));
}
