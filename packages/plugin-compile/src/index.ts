export { compileGuest, CompileGuestError } from "./compile";
export { attachWasm, encodeWasmBase64, packPlugin } from "./pack";
export { inspectGuestWasm } from "./inspect";
export { previewGuest } from "./preview";

/** 桌面沙箱与本包编译器共用的 guest ABI。 */
export const GUEST_ABI = {
  memory: "memory",
  execute: "plugin_execute",
  executeMulti: "(i32,i32)->(i32,i32)",
  executePacked: "(i32,i32)->i64  // (ptr << 32) | len",
  request: { operation: "string", input: "object" },
  response: { output: "object", logs: "string[]?" },
} as const;
