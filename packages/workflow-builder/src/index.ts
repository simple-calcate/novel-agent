export interface WorkflowDefinition {
  id: string;
  name: string;
  enabled: boolean;
  trigger: string;
  conditions: WorkflowCondition[];
  actions: WorkflowAction[];
  priority: number;
  cooldownMs: number;
}

export interface WorkflowCondition {
  path: string;
  operator: "eq" | "notEq" | "gt" | "gte" | "lt" | "lte" | "contains" | "exists";
  value: unknown;
}

export type WorkflowAction =
  | { type: "saveDocument" }
  | { type: "rebuildIndex" }
  | { type: "checkContinuity" }
  | { type: "generateContinuation"; maxTokens: number }
  | { type: "createBackup" }
  | { type: "runAgent"; prompt: string }
  | { type: "runPluginOperation"; pluginId: string; operation: string; input?: unknown };

export function createIdleWorkflow(id: string, idleMs = 1800): WorkflowDefinition {
  return {
    id,
    name: "停笔后保存并刷新索引",
    enabled: true,
    trigger: "editor.idle",
    conditions: [
      { path: "idleMs", operator: "gte", value: idleMs },
      { path: "composing", operator: "eq", value: false },
    ],
    actions: [{ type: "saveDocument" }, { type: "rebuildIndex" }],
    priority: 100,
    cooldownMs: 5000,
  };
}

export function createChapterWorkflow(id: string): WorkflowDefinition {
  return {
    id,
    name: "新章节后生成大纲草稿",
    enabled: true,
    trigger: "chapter.created",
    conditions: [{ path: "source", operator: "notEq", value: "import" }],
    actions: [{ type: "runAgent", prompt: "基于上一章与新章节标题生成大纲草稿" }],
    priority: 50,
    cooldownMs: 0,
  };
}
