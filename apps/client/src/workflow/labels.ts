export const OPERATION_LABELS: Record<string, string> = {
  "document.save": "保存文档",
  "index.rebuild": "刷新索引",
  "continuity.check": "连续性检查",
  "backup.create": "创建备份",
  "agent.continuation": "AI 续写",
  "agent.run": "运行 Agent",
  "plugin.operation": "插件操作",
  "block.save": "保存块",
  "block.edit": "编辑块",
  "training.export": "导出训练数据",
};

export function operationLabel(operation: string): string {
  return OPERATION_LABELS[operation] ?? operation;
}
