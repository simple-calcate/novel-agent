# 性能基线

## 目标

| 指标 | 目标 | 测量方式 |
|------|------|----------|
| 长章节输入延迟 | < 16ms | 编辑器 onUpdate 到渲染完成 |
| 百万字项目打开 | < 3s | 从启动到章节树可交互 |
| 全文搜索响应 | < 300ms | FTS5 查询 10 万字语料 |
| 增量索引 | < 50ms/章节 | 保存后索引更新时间 |
| 队列吞吐 | > 100 jobs/s | 批量 enqueue 测试 |
| 内存占用 | < 200MB | 打开 100 万字项目 |
| 冷启动 | < 2s | 从进程创建到 UI 就绪 |
| Android 低内存 | 不崩溃 | 1GB RAM 设备打开 10 万字 |

## 测试

```bash
# Rust 测试
cargo test --workspace

# 前端测试
pnpm -r test

# 类型检查
pnpm -r typecheck

# 构建
pnpm -r build
cargo build --release
```
