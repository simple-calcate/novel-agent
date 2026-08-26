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
- 桌面端扩展性最强；`plugin.operation` 在清单带 `wasmBase64` 时用 wasmi 执行 guest（导出 `memory` 与 `plugin_execute(i32,i32)->(i32,i32)`，JSON in/out）。无 WASI、无宿主导入、有燃料上限。
- Android 端忽略 WASM，退回内置执行器。
- 需要维护清单 schema、SDK、权限模型和示例插件。
