export type GuestExportKind = "func" | "table" | "memory" | "global" | "unknown";

export type GuestWasmInfo = {
  imports: Array<{ module: string; name: string }>;
  exports: Array<{ name: string; kind: GuestExportKind }>;
};

const KIND_NAMES: GuestExportKind[] = ["func", "table", "memory", "global"];

/** 解析 wasm 二进制的 import / export 段，用来确认没有 WASI、有 memory 与 plugin_execute。 */
export function inspectGuestWasm(bytes: Uint8Array): GuestWasmInfo {
  if (
    bytes.length < 8 ||
    bytes[0] !== 0x00 ||
    bytes[1] !== 0x61 ||
    bytes[2] !== 0x73 ||
    bytes[3] !== 0x6d
  ) {
    throw new Error("不是 WASM 模块");
  }
  let offset = 8;
  const imports: GuestWasmInfo["imports"] = [];
  const exports: GuestWasmInfo["exports"] = [];
  while (offset < bytes.length) {
    const id = bytes[offset++];
    const size = readU32(bytes, offset);
    offset = size.next;
    const end = offset + size.value;
    if (end > bytes.length) break;
    if (id === 2) parseImportSection(bytes, offset, end, imports);
    if (id === 7) parseExportSection(bytes, offset, end, exports);
    offset = end;
  }
  return { imports, exports };
}

function parseImportSection(
  bytes: Uint8Array,
  start: number,
  end: number,
  imports: GuestWasmInfo["imports"],
): void {
  let offset = start;
  const count = readU32(bytes, offset);
  offset = count.next;
  for (let i = 0; i < count.value && offset < end; i++) {
    const moduleName = readName(bytes, offset);
    offset = moduleName.next;
    const field = readName(bytes, offset);
    offset = field.next;
    imports.push({ module: moduleName.value, name: field.value });
    offset = skipImportDesc(bytes, offset);
  }
}

function parseExportSection(
  bytes: Uint8Array,
  start: number,
  end: number,
  exports: GuestWasmInfo["exports"],
): void {
  let offset = start;
  const count = readU32(bytes, offset);
  offset = count.next;
  for (let i = 0; i < count.value && offset < end; i++) {
    const name = readName(bytes, offset);
    offset = name.next;
    const kindByte = bytes[offset++] ?? 255;
    const kind = KIND_NAMES[kindByte] ?? "unknown";
    const index = readU32(bytes, offset);
    offset = index.next;
    exports.push({ name: name.value, kind });
  }
}

function skipImportDesc(bytes: Uint8Array, offset: number): number {
  const kind = bytes[offset++] ?? 255;
  if (kind === 0) {
    return readU32(bytes, offset).next;
  }
  if (kind === 1) {
    offset += 1;
    return skipLimits(bytes, offset);
  }
  if (kind === 2) {
    return skipLimits(bytes, offset);
  }
  if (kind === 3) {
    return offset + 2;
  }
  return offset;
}

function skipLimits(bytes: Uint8Array, offset: number): number {
  const flags = bytes[offset++] ?? 0;
  offset = readU32(bytes, offset).next;
  if (flags & 1) {
    offset = readU32(bytes, offset).next;
  }
  return offset;
}

function readU32(bytes: Uint8Array, offset: number): { value: number; next: number } {
  let result = 0;
  let shift = 0;
  let pos = offset;
  while (pos < bytes.length) {
    const byte = bytes[pos++];
    result |= (byte & 0x7f) << shift;
    if ((byte & 0x80) === 0) break;
    shift += 7;
  }
  return { value: result >>> 0, next: pos };
}

function readName(bytes: Uint8Array, offset: number): { value: string; next: number } {
  const len = readU32(bytes, offset);
  const start = len.next;
  const end = start + len.value;
  const value = new TextDecoder().decode(bytes.slice(start, end));
  return { value, next: end };
}
