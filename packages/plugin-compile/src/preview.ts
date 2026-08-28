/**
 * 在 Node / 浏览器里预览 guest，协议与桌面沙箱相同：JSON 写到已有内存之后。
 * 这不是 wasmi，不能代替燃料上限与无导入检查；正式执行仍走桌面宿主。
 */
export function previewGuest(
  wasm: Uint8Array,
  operation: string,
  input: unknown,
): { output: unknown; logs?: string[] } {
  const module = new WebAssembly.Module(Uint8Array.from(wasm));
  const instance = new WebAssembly.Instance(module, {});
  const memory = instance.exports.memory as WebAssembly.Memory | undefined;
  const execute = instance.exports.plugin_execute as
    | ((ptr: number, len: number) => bigint | number | number[])
    | undefined;
  if (!memory || typeof execute !== "function") {
    throw new Error("guest 必须导出 memory 与 plugin_execute");
  }
  const request = new TextEncoder().encode(JSON.stringify({ operation, input }));
  const ptr = memory.buffer.byteLength;
  const needed = ptr + request.length;
  const page = 65536;
  const havePages = memory.buffer.byteLength / page;
  const needPages = Math.ceil(needed / page);
  if (needPages > havePages) {
    memory.grow(needPages - havePages);
  }
  new Uint8Array(memory.buffer, ptr, request.length).set(request);
  const result = execute(ptr, request.length);
  const { outPtr, outLen } = unpackExecuteResult(result);
  const bytes = new Uint8Array(memory.buffer, outPtr, outLen);
  return JSON.parse(new TextDecoder().decode(bytes)) as { output: unknown; logs?: string[] };
}

function unpackExecuteResult(result: bigint | number | number[]): { outPtr: number; outLen: number } {
  if (typeof result === "bigint") {
    return {
      outPtr: Number(result >> 32n),
      outLen: Number(result & 0xffffffffn),
    };
  }
  if (Array.isArray(result) && result.length >= 2) {
    return { outPtr: Number(result[0]), outLen: Number(result[1]) };
  }
  throw new Error(`plugin_execute 返回了无法识别的值：${typeof result}`);
}
