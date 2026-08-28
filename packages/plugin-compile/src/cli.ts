import { compileGuest } from "./compile.ts";
import { packPlugin } from "./pack.ts";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const pkgRoot = path.resolve(here, "..");

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const example = flag(args, "--example") ?? (args[0] === "hello-names" ? "hello-names" : null);
  const guest =
    flag(args, "--guest") ??
    (example === "hello-names" ? path.join(pkgRoot, "examples/hello-names.ts") : null);
  const wasmOut = flag(args, "--wasm");
  const jsonOut = flag(args, "--out");
  if (!guest) {
    process.stderr.write(
      "用法: moshu-plugin-compile --guest assembly/index.ts [--out plugin.json] [--wasm plugin.wasm]\n" +
        "      moshu-plugin-compile --example hello-names --wasm build/hello-names.wasm\n",
    );
    process.exitCode = 1;
    return;
  }
  const wasm = await compileGuest(guest);
  if (wasmOut) {
    mkdirSync(path.dirname(path.resolve(wasmOut)), { recursive: true });
    writeFileSync(path.resolve(wasmOut), wasm);
  }
  if (jsonOut) {
    const { helloNames } = await import("@novel-agent/plugin-sdk");
    mkdirSync(path.dirname(path.resolve(jsonOut)), { recursive: true });
    writeFileSync(path.resolve(jsonOut), packPlugin(helloNames, wasm), "utf8");
  }
  if (!wasmOut && !jsonOut) {
    process.stdout.write(`${wasm.length} bytes\n`);
  }
}

function flag(args: string[], name: string): string | null {
  const index = args.indexOf(name);
  if (index < 0) return null;
  return args[index + 1] ?? null;
}

void main();
