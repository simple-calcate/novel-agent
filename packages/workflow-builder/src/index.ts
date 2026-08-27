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

export class WorkflowDefinitionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "WorkflowDefinitionError";
  }
}

/** 作者侧声明式工作流入口：不写 WASM，只声明触发器与动作。 */
export function defineWorkflow(definition: WorkflowDefinition): WorkflowDefinition {
  const id = definition.id?.trim() ?? "";
  const name = definition.name?.trim() ?? "";
  const trigger = definition.trigger?.trim() ?? "";
  if (!id) {
    throw new WorkflowDefinitionError("workflow id 不能为空");
  }
  if (!name) {
    throw new WorkflowDefinitionError("workflow name 不能为空");
  }
  if (!trigger) {
    throw new WorkflowDefinitionError("workflow trigger 不能为空");
  }
  if (!definition.actions?.length) {
    throw new WorkflowDefinitionError("至少一条 action");
  }
  return {
    ...definition,
    id,
    name,
    trigger,
    conditions: definition.conditions ?? [],
    enabled: definition.enabled ?? true,
    priority: definition.priority ?? 100,
    cooldownMs: definition.cooldownMs ?? 0,
  };
}
