# ADR 0004: 分层插件与 WASM 沙箱

## 状态
已接受

## 背景
用户需要高度自定义，但第三方代码不能直接访问文件、网络或密钥。

## 决策
- 第一层：声明式工作流，所有平台可用。
- 第二层：可信 Rust 操作，随应用编译。
- 第三层：Linux/Windows 上的 WASM 操作插件。
- Android 首版仅支持声明式工作流与内置操作。

## 后果
- 桌面端扩展性最强；`plugin.operation` 在清单带 `wasmBase64` 时用 wasmi 执行 guest。导出 `memory`；`plugin_execute` 为 `(i32,i32)->(i32,i32)` 或 packed i64 `(ptr << 32) | len`。宿主把请求 JSON 写在已有线性内存之后，避免盖掉 guest 静态数据。无 WASI、无宿主导入、有燃料上限。
- 第三方 guest 用 MIT 包 `@novel-agent/plugin-compile`（AssemblyScript）编成无导入 wasm32。这不是完整 TypeScript，也不是插件商店。
- Android 端忽略 WASM，退回内置执行器。
- 需要维护清单 schema、SDK、编译脚手架、权限模型和示例插件。
