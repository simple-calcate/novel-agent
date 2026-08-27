/// <reference types="assemblyscript/std/assembly" />

/** 读入 UTF-8 请求，把 UTF-8 响应写到输入后面，返回 packed i64：(ptr << 32) | len。 */
export function runExecute(ptr: i32, len: i32, fn: (request: string) => string): i64 {
  const request = String.UTF8.decodeUnsafe(usize(ptr), usize(len), false);
  const response = fn(request);
  const outPtr = ptr + len;
  const outLen = utf8ByteLength(response);
  const end = outPtr + outLen;
  const neededPages = (end + 65535) >> 16;
  const have = memory.size();
  if (neededPages > have) {
    const grown = memory.grow(neededPages - have);
    if (grown < 0) {
      return 0;
    }
  }
  utf8Write(response, outPtr);
  return (i64(outPtr) << 32) | i64(outLen);
}

function utf8ByteLength(value: string): i32 {
  let n = 0;
  for (let i = 0; i < value.length; i++) {
    const c = value.charCodeAt(i);
    if (c < 0x80) n += 1;
    else if (c < 0x800) n += 2;
    else if (c >= 0xd800 && c <= 0xdbff && i + 1 < value.length) {
      n += 4;
      i += 1;
    } else n += 3;
  }
  return n;
}

function utf8Write(value: string, ptr: i32): void {
  let offset = 0;
  for (let i = 0; i < value.length; i++) {
    const c = value.charCodeAt(i);
    if (c < 0x80) {
      store<u8>(ptr + offset, <u8>c);
      offset += 1;
    } else if (c < 0x800) {
      store<u8>(ptr + offset, <u8>(0xc0 | (c >> 6)));
      store<u8>(ptr + offset + 1, <u8>(0x80 | (c & 0x3f)));
      offset += 2;
    } else if (c >= 0xd800 && c <= 0xdbff && i + 1 < value.length) {
      const c2 = value.charCodeAt(++i);
      const cp = 0x10000 + (((c & 0x3ff) << 10) | (c2 & 0x3ff));
      store<u8>(ptr + offset, <u8>(0xf0 | (cp >> 18)));
      store<u8>(ptr + offset + 1, <u8>(0x80 | ((cp >> 12) & 0x3f)));
      store<u8>(ptr + offset + 2, <u8>(0x80 | ((cp >> 6) & 0x3f)));
      store<u8>(ptr + offset + 3, <u8>(0x80 | (cp & 0x3f)));
      offset += 4;
    } else {
      store<u8>(ptr + offset, <u8>(0xe0 | (c >> 12)));
      store<u8>(ptr + offset + 1, <u8>(0x80 | ((c >> 6) & 0x3f)));
      store<u8>(ptr + offset + 2, <u8>(0x80 | (c & 0x3f)));
      offset += 3;
    }
  }
}
