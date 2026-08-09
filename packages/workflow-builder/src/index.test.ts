import { describe, expect, it } from "vitest";
import { createIdleWorkflow, createChapterWorkflow } from "./index";

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
});
