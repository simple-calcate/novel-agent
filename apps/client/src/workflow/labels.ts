import type { WorkflowAction } from "@novel-agent/workflow-builder";
import { t } from "../i18n";

export function operationLabel(operation: string): string {
  switch (operation) {
    case "document.save":
      return t("workflow.opSave");
    case "index.rebuild":
      return t("workflow.opIndex");
    case "continuity.check":
      return t("workflow.opCheck");
    case "backup.create":
      return t("workflow.opBackup");
    case "agent.continuation":
      return t("workflow.opContinue");
    case "agent.run":
      return t("workflow.opAgent");
    case "plugin.operation":
      return t("workflow.opPlugin");
    case "block.save":
      return t("workflow.opBlockSave");
    case "block.edit":
      return t("workflow.opBlockEdit");
    case "training.export":
      return t("workflow.opExport");
    default:
      return operation;
  }
}

export function actionLabel(action: WorkflowAction): string {
  switch (action.type) {
    case "saveDocument":
      return t("workflow.opSave");
    case "rebuildIndex":
      return t("workflow.opIndex");
    case "checkContinuity":
      return t("workflow.opCheck");
    case "generateContinuation":
    case "runAgent":
      return t("workflow.opContinue");
    case "createBackup":
      return t("workflow.opBackup");
    case "runPluginOperation":
      return action.pluginId === "hello-names" ? t("plugin.helloNames") : t("workflow.opPlugin");
  }
}

export function workflowTemplateName(id: string, fallback: string): string {
  switch (id) {
    case "idle-save":
      return t("workflow.templateIdle");
    case "chapter-outline":
      return t("workflow.templateChapter");
    case "paragraph-check":
      return t("workflow.templateParagraph");
    case "save-check":
      return t("workflow.templateSave");
    case "name-count":
      return t("workflow.templateNames");
    default:
      return fallback;
  }
}

export function triggerLabel(trigger: string): string {
  switch (trigger) {
    case "editor.idle":
      return t("workflow.triggerIdle");
    case "chapter.created":
      return t("workflow.triggerChapter");
    case "paragraph.created":
      return t("workflow.triggerParagraph");
    case "document.saved":
      return t("workflow.triggerSaved");
    case "manual":
      return t("workflow.triggerManual");
    default:
      return trigger;
  }
}

export function jobStatusLabel(status: string): string {
  switch (status) {
    case "pending":
      return t("workflow.statusPending");
    case "blocked":
      return t("workflow.statusBlocked");
    case "running":
      return t("workflow.statusRunning");
    case "succeeded":
      return t("workflow.statusSucceeded");
    case "failed":
      return t("workflow.statusFailed");
    case "cancelled":
      return t("workflow.statusCancelled");
    case "deadLetter":
      return t("workflow.statusDeadLetter");
    default:
      return status;
  }
}
