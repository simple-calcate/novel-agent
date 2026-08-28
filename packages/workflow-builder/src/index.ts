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

export function createParagraphCheckWorkflow(id: string): WorkflowDefinition {
  return {
    id,
    name: "新段落超过 200 字时检查设定",
    enabled: true,
    trigger: "paragraph.created",
    conditions: [{ path: "wordCount", operator: "gte", value: 200 }],
    actions: [{ type: "checkContinuity" }],
    priority: 80,
    cooldownMs: 10_000,
  };
}

export function createSaveCheckWorkflow(id: string): WorkflowDefinition {
  return {
    id,
    name: "保存后运行连续性检查",
    enabled: true,
    trigger: "document.saved",
    conditions: [],
    actions: [{ type: "checkContinuity" }],
    priority: 90,
    cooldownMs: 5000,
  };
}

/** 手动点播放时跑打包的人名点名。不是可视化编辑器。 */
export function createNameCountWorkflow(id: string): WorkflowDefinition {
  return {
    id,
    name: "点名当前章",
    enabled: true,
    trigger: "manual",
    conditions: [],
    actions: [
      { type: "runPluginOperation", pluginId: "hello-names", operation: "count-names" },
    ],
    priority: 40,
    cooldownMs: 0,
  };
}

export function actionToToolId(action: WorkflowAction): string {
  switch (action.type) {
    case "saveDocument":
      return "document.save";
    case "rebuildIndex":
      return "index.rebuild";
    case "checkContinuity":
      return "continuity.check";
    case "generateContinuation":
    case "runAgent":
      return "agent.continuation";
    case "createBackup":
      return "backup.create";
    case "runPluginOperation":
      return "plugin.operation";
  }
}

export function bundledWorkflowTemplates(): WorkflowDefinition[] {
  return [
    createIdleWorkflow("idle-save"),
    createChapterWorkflow("chapter-outline"),
    createParagraphCheckWorkflow("paragraph-check"),
    createSaveCheckWorkflow("save-check"),
    createNameCountWorkflow("name-count"),
  ];
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
