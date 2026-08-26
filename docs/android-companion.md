# Android Companion 实现说明

## 范围

- 阅读：完整项目树、章节正文、注释查看
- 轻编辑：纯文本编辑、自动保存、Revision 提交
- 任务查看：队列状态、工作流历史、Agent 运行记录
- 同步：手动触发、系统允许的后台同步

## 降级策略

- 无 WASM 插件：仅支持声明式工作流与内置操作
- 无后台 Agent：长任务通过 WorkManager/前台服务
- 语义检索：复用桌面生成的向量，不可用时退化为 FTS

## 构建

CI 对无 C 依赖的 crate 执行：

```bash
cargo check --target aarch64-linux-android \
  -p novel-domain \
  -p novel-kernel \
  -p novel-story-model \
  -p novel-feedback-memory \
  -p novel-context-hints
```

完整 APK 仍需要 Android SDK、NDK 和 Rust `aarch64-linux-android` target：

```bash
cd apps/client
pnpm tauri android init
pnpm tauri android build
```
