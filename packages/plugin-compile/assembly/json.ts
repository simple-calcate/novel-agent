/// <reference types="assemblyscript/std/assembly" />

/** 取出 JSON 对象里某个字符串字段；没有则空串。 */
export function jsonString(source: string, key: string): string {
  const raw = extractValue(source, key);
  if (raw.length == 0 || raw.charCodeAt(0) != 34) return "";
  return parseJsonString(raw);
}

/** 取出 JSON 对象字段，原样返回（含花括号）。 */
export function jsonObject(source: string, key: string): string {
  const raw = extractValue(source, key);
  if (raw.length == 0 || raw.charCodeAt(0) != 123) return "{}";
  return raw;
}

/** 取出字符串数组字段。 */
export function jsonStringArray(source: string, key: string): string[] {
  const raw = extractValue(source, key);
  const out = new Array<string>();
  if (raw.length == 0 || raw.charCodeAt(0) != 91) return out;
  let i = 1;
  while (i < raw.length) {
    while (i < raw.length && (isWs(raw.charCodeAt(i)) || raw.charCodeAt(i) == 44)) i++;
    if (i >= raw.length || raw.charCodeAt(i) == 93) break;
    if (raw.charCodeAt(i) != 34) break;
    const sliced = sliceJsonString(raw, i);
    out.push(parseJsonString(sliced));
    i += sliced.length;
  }
  return out;
}

export function jsonEscape(value: string): string {
  let out = "";
  for (let i = 0; i < value.length; i++) {
    const c = value.charCodeAt(i);
    if (c == 34) out += '\\"';
    else if (c == 92) out += "\\\\";
    else if (c == 10) out += "\\n";
    else if (c == 13) out += "\\r";
    else if (c == 9) out += "\\t";
    else out += String.fromCharCode(c);
  }
  return out;
}

function extractValue(source: string, key: string): string {
  const afterColon = findKey(source, key);
  if (afterColon < 0) return "";
  return parseValue(source, afterColon);
}

function findKey(source: string, key: string): i32 {
  const needle = '"' + key + '"';
  let i = 0;
  let inString = false;
  let escape = false;
  while (i < source.length) {
    const c = source.charCodeAt(i);
    if (inString) {
      if (escape) escape = false;
      else if (c == 92) escape = true;
      else if (c == 34) inString = false;
      i++;
      continue;
    }
    if (c == 34) {
      if (i + needle.length <= source.length && source.substring(i, i + needle.length) == needle) {
        let j = i + needle.length;
        while (j < source.length && isWs(source.charCodeAt(j))) j++;
        if (j < source.length && source.charCodeAt(j) == 58) return j + 1;
      }
      inString = true;
    }
    i++;
  }
  return -1;
}

function parseValue(source: string, start: i32): string {
  let i = start;
  while (i < source.length && isWs(source.charCodeAt(i))) i++;
  if (i >= source.length) return "";
  const c = source.charCodeAt(i);
  if (c == 34) return sliceJsonString(source, i);
  if (c == 123) return sliceBalanced(source, i, 123, 125);
  if (c == 91) return sliceBalanced(source, i, 91, 93);
  let j = i;
  while (j < source.length && !isValueEnd(source.charCodeAt(j))) j++;
  return source.substring(i, j);
}

function sliceJsonString(source: string, start: i32): string {
  let i = start + 1;
  let escape = false;
  while (i < source.length) {
    const c = source.charCodeAt(i);
    if (escape) escape = false;
    else if (c == 92) escape = true;
    else if (c == 34) return source.substring(start, i + 1);
    i++;
  }
  return source.substring(start);
}

function sliceBalanced(source: string, start: i32, open: i32, close: i32): string {
  let depth = 0;
  let i = start;
  let inString = false;
  let escape = false;
  while (i < source.length) {
    const c = source.charCodeAt(i);
    if (inString) {
      if (escape) escape = false;
      else if (c == 92) escape = true;
      else if (c == 34) inString = false;
      i++;
      continue;
    }
    if (c == 34) inString = true;
    else if (c == open) depth++;
    else if (c == close) {
      depth--;
      if (depth == 0) return source.substring(start, i + 1);
    }
    i++;
  }
  return source.substring(start);
}

function parseJsonString(quoted: string): string {
  if (quoted.length < 2) return "";
  const inner = quoted.substring(1, quoted.length - 1);
  let out = "";
  for (let i = 0; i < inner.length; i++) {
    const c = inner.charCodeAt(i);
    if (c != 92 || i + 1 >= inner.length) {
      out += String.fromCharCode(c);
      continue;
    }
    const n = inner.charCodeAt(++i);
    if (n == 34) out += '"';
    else if (n == 92) out += "\\";
    else if (n == 110) out += "\n";
    else if (n == 114) out += "\r";
    else if (n == 116) out += "\t";
    else if (n == 117 && i + 4 < inner.length) {
      const hex = inner.substring(i + 1, i + 5);
      out += String.fromCharCode(i32.parse(hex, 16));
      i += 4;
    } else out += String.fromCharCode(n);
  }
  return out;
}

function isWs(c: i32): bool {
  return c == 32 || c == 9 || c == 10 || c == 13;
}

function isValueEnd(c: i32): bool {
  return isWs(c) || c == 44 || c == 125 || c == 93;
}
