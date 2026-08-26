# ADR 0006: Android 首版策略

## 状态
已接受

## 背景
Android 的 WebView、后台任务和 WASM 运行时支持弱于桌面。

## 决策
- Android 首版提供阅读、轻编辑、注释、任务查看和手动同步。
- 不承诺任意第三方 WASM 插件或无限后台 Agent。
- 延期任务使用 WorkManager/前台服务。
- 保留未来替换为独立 Kotlin/Compose 壳的边界。

## 后果
- 首版功能明确、风险可控。
- 与桌面端存在能力差异，需在 UI 中明确提示。
- 架构需允许后续 Android 原生壳替换。
- CI `android-build` 对无 C 依赖 crate（`novel-domain`、`novel-kernel`、`novel-story-model`、`novel-feedback-memory`、`novel-context-hints`）做 `aarch64-linux-android` 的 `cargo check`。完整 APK/NDK 仍未装。
