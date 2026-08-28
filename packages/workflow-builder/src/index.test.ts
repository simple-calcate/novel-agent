import { describe, expect, it } from "vitest";
import {
  actionToToolId,
  bundledWorkflowTemplates,
  createIdleWorkflow,
  createChapterWorkflow,
  defineWorkflow,
  WorkflowDefinitionError,
} from "./index";

describe("workflow templates", () => {
  it("idle workflow has correct trigger and conditions", () => {
    const wf = createIdleWorkflow("wf-1");
    expect(wf.trigger).toBe("editor.idle");
    expect(wf.conditions).toHaveLength(2);
    expect(wf.actions).toHaveLength(2);
    expect(wf.actions[0].type).toBe("saveDocument");
  });

  it("chapter workflow excludes import source", () => {
    const wf = createChapterWorkflow("wf-2");
    expect(wf.trigger).toBe("chapter.created");
    expect(wf.conditions[0].operator).toBe("notEq");
    expect(wf.conditions[0].value).toBe("import");
  });

  it("maps actions to kernel tool ids", () => {
    expect(actionToToolId({ type: "saveDocument" })).toBe("document.save");
    expect(actionToToolId({ type: "checkContinuity" })).toBe("continuity.check");
    expect(
      actionToToolId({ type: "runPluginOperation", pluginId: "hello-names", operation: "count-names" }),
    ).toBe("plugin.operation");
  });

  it("bundled templates are the ones shown in the app", () => {
    expect(bundledWorkflowTemplates().map((item) => item.trigger)).toEqual([
      "editor.idle",
      "chapter.created",
      "paragraph.created",
      "document.saved",
      "manual",
    ]);
    expect(bundledWorkflowTemplates().at(-1)?.actions[0]).toEqual({
      type: "runPluginOperation",
      pluginId: "hello-names",
      operation: "count-names",
    });
  });

  it("defineWorkflow rejects empty actions", () => {
    expect(() =>
      defineWorkflow({
        id: "wf-3",
        name: "空",
        enabled: true,
        trigger: "editor.idle",
        conditions: [],
        actions: [],
        priority: 1,
        cooldownMs: 0,
      }),
    ).toThrow(WorkflowDefinitionError);
  });
});
