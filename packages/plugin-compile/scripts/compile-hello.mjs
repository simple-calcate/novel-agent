#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import * as asc from "assemblyscript/asc";

const pkgRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = path.resolve(pkgRoot, "../..");
const entry = path.join(pkgRoot, "examples/hello-names.ts");
const baseManifest = path.join(repoRoot, "plugins/hello-names/plugin.base.json");
const packedOut = path.join(repoRoot, "plugins/hello-names/plugin.json");
const wasmOut = process.argv[2] ?? path.join(pkgRoot, "build/hello-names.wasm");

const dir = fs.mkdtempSync(path.join(os.tmpdir(), "moshu-plugin-"));
const outFile = path.join(dir, "plugin.wasm");
const log = [];
const sink = {
  write(chunk) {
    log.push(typeof chunk === "string" ? chunk : new TextDecoder().decode(chunk));
  },
};

const result = await asc.main(
  [
    entry,
    "--outFile",
    outFile,
    "--runtime",
    "stub",
    "--use",
    "abort=",
    "--optimizeLevel",
    "3",
    "--shrinkLevel",
    "2",
    "--noAssert",
    "--disable",
    "simd",
  ],
  { stdout: sink, stderr: sink },
);

if (result.error) {
  process.stderr.write(`${result.error.message}\n${log.join("")}\n`);
  process.exit(1);
}

const wasm = fs.readFileSync(outFile);
fs.mkdirSync(path.dirname(path.resolve(wasmOut)), { recursive: true });
fs.writeFileSync(path.resolve(wasmOut), wasm);

const manifest = JSON.parse(fs.readFileSync(baseManifest, "utf8"));
manifest.wasmBase64 = Buffer.from(wasm).toString("base64");
fs.mkdirSync(path.dirname(packedOut), { recursive: true });
fs.writeFileSync(packedOut, `${JSON.stringify(manifest, null, 2)}\n`);
fs.rmSync(dir, { recursive: true, force: true });
process.stdout.write(`${path.resolve(wasmOut)}\n${packedOut}\n`);
