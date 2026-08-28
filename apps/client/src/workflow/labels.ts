import type { WorkflowAction } from "@novel-agent/workflow-builder";

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

export function actionLabel(action: WorkflowAction): string {
  switch (action.type) {
    case "saveDocument":
      return "保存文档";
    case "rebuildIndex":
      return "刷新索引";
    case "checkContinuity":
      return "连续性检查";
    case "generateContinuation":
    case "runAgent":
      return "AI 续写";
    case "createBackup":
      return "创建备份";
    case "runPluginOperation":
      return action.pluginId === "hello-names" ? "人名点名" : "插件操作";
  }
}
