/// <reference types="assemblyscript/std/assembly" />

import { runExecute } from "../assembly/execute";
import { jsonEscape, jsonObject, jsonString, jsonStringArray } from "../assembly/json";

/** 与 `@novel-agent/plugin-sdk` 的 hello-names 对应的 guest。语法是 AssemblyScript。 */
export function plugin_execute(ptr: i32, len: i32): i64 {
  return runExecute(ptr, len, dispatch);
}

function dispatch(request: string): string {
  const operation = jsonString(request, "operation");
  const input = jsonObject(request, "input");
  if (operation == "count-names") {
    return countNames(input);
  }
  return '{"output":{"error":"unknown operation"},"logs":["wasm"]}';
}

function countNames(input: string): string {
  const selection = jsonString(input, "selection");
  const names = jsonStringArray(input, "names");
  let body = "";
  for (let i = 0; i < names.length; i++) {
    const name = names[i];
    if (i > 0) body += ",";
    body += '"' + jsonEscape(name) + '":' + countOccurrences(selection, name).toString();
  }
  return '{"output":{"counts":{' + body + '}},"logs":["hello-names"]}';
}

function countOccurrences(selection: string, name: string): i32 {
  if (name.length == 0) return 0;
  let found = 0;
  let from = 0;
  while (from <= selection.length) {
    const index = selection.indexOf(name, from);
    if (index < 0) break;
    found += 1;
    from = index + name.length;
  }
  return found;
}
