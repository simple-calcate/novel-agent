export class CompileGuestError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CompileGuestError";
  }
}

/**
 * 把 AssemblyScript guest（导出 `plugin_execute(i32,i32)->i64` 与 `memory`）编成
 * 无导入的 wasm32 模块。不能有 WASI，也不能 import host。
 */
export async function compileGuest(entryFile: string): Promise<Uint8Array> {
  const fs = await import("node:fs");
  const os = await import("node:os");
  const path = await import("node:path");
  const asc = await import("assemblyscript/asc");

  const entry = path.resolve(entryFile);
  if (!fs.existsSync(entry)) {
    throw new CompileGuestError(`找不到 guest 入口：${entry}`);
  }

  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "moshu-plugin-"));
  const outFile = path.join(dir, "plugin.wasm");
  const log: string[] = [];
  const sink = {
    write(chunk: string | Uint8Array): void {
      log.push(typeof chunk === "string" ? chunk : new TextDecoder().decode(chunk));
    },
  };

  try {
    const argv = [
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
    ];
    const result = await asc.main(argv, {
      stdout: sink,
      stderr: sink,
    });
    if (result.error) {
      throw new CompileGuestError(
        `AssemblyScript 编译失败：${result.error.message}\n${log.join("")}`.trim(),
      );
    }
    if (!fs.existsSync(outFile)) {
      throw new CompileGuestError(`编译器没有写出 ${outFile}\n${log.join("")}`.trim());
    }
    return new Uint8Array(fs.readFileSync(outFile));
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}
