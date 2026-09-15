import { FileDown, Play, Workflow } from "lucide-react";
import { useEffect, useState } from "react";
import {
  actionToToolId,
  bundledWorkflowTemplates,
  type WorkflowAction,
  type WorkflowDefinition,
} from "@novel-agent/workflow-builder";
import { libraryApi } from "../api";
import { formatPluginResult, pluginDisplayName, splitNames } from "../plugins/format";
import {
  actionLabel,
  jobStatusLabel,
  operationLabel,
  triggerLabel,
  workflowTemplateName,
} from "../workflow/labels";
import { joinList, t, useI18n } from "../i18n";

interface Props {
  jobs: Array<{ id: string; operation: string; status: string }>;
  queueReady: boolean;
  chapterText: string;
  characterNames: string[];
  onRun: (operation: string, label: string) => void;
}

const templates = bundledWorkflowTemplates();

export function WorkflowPanel({
  jobs,
  queueReady,
  chapterText,
  characterNames,
  onRun,
}: Props) {
  const { t: tx } = useI18n();
  const [pending, setPending] = useState(0);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void libraryApi
      .pendingOutboxCount()
      .then((count) => {
        if (!cancelled) setPending(count);
      })
      .catch(() => {
        if (!cancelled) setPending(0);
      });
    return () => {
      cancelled = true;
    };
  }, [jobs]);

  async function flushJournal() {
    setBusy(true);
    setMessage(null);
    try {
      const result = await libraryApi.flushOutboxJournal();
      setPending(0);
      const location = result.path ? ` → ${result.path}` : "";
      setMessage(tx("workflow.flushed", { count: result.written, location, note: result.note }));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : tx("workflow.flushFail"));
    } finally {
      setBusy(false);
    }
  }

  async function play(template: WorkflowDefinition) {
    setBusy(true);
    setMessage(null);
    try {
      const notes: string[] = [];
      for (const action of template.actions) {
        if (action.type === "runPluginOperation") {
          notes.push(await runPluginAction(action, chapterText, characterNames));
        } else {
          onRun(actionToToolId(action), workflowTemplateName(template.id, template.name));
        }
      }
      if (notes.length > 0) {
        setMessage(notes.join("\n"));
      }
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="panel-content">
      <div className="panel-heading">
        <h3>{tx("workflow.heading")}</h3>
      </div>
      <p className="panel-muted">{tx("workflow.lead")}</p>

      <div className="outbox-journal">
        <div className="outbox-journal-copy">
          <strong>{tx("workflow.pending", { count: pending })}</strong>
          <p>{tx("workflow.journalHint")}</p>
        </div>
        <button className="mini-button" onClick={() => void flushJournal()} disabled={busy}>
          <FileDown size={12} />
          {busy ? tx("workflow.flushing") : tx("workflow.flush")}
        </button>
      </div>
      {message && <pre className="plugin-result workflow-run-note">{message}</pre>}

      <div className="workflow-list">
        {templates.map((template) => (
          <div key={template.id} className="workflow-item">
            <div className="workflow-icon">
              <Workflow size={14} />
            </div>
            <div className="workflow-body">
              <div className="workflow-name">{workflowTemplateName(template.id, template.name)}</div>
              <div className="workflow-trigger">{triggerLabel(template.trigger)}</div>
              <div className="workflow-actions">
                {template.actions.map((action, index) => (
                  <span key={`${template.id}-${index}`}>{actionLabel(action)}</span>
                ))}
              </div>
            </div>
            <button
              className="mini-button"
              disabled={busy}
              onClick={() => void play(template)}
              title={tx("workflow.play")}
            >
              <Play size={12} />
            </button>
          </div>
        ))}
      </div>

      <h3 className="jobs-heading">{tx("workflow.jobs")}</h3>
      <div className="job-list">
        {!queueReady && <div className="empty-state">{tx("workflow.queueOffline")}</div>}
        {queueReady && jobs.length === 0 && <div className="empty-state">{tx("workflow.noJobs")}</div>}
        {jobs.map((job) => (
          <div key={job.id} className="job-item">
            <span className={`job-status ${job.status === "succeeded" ? "done" : ""} ${job.status}`} />
            <span className="job-label">{operationLabel(job.operation)}</span>
            <span className="job-state">{jobStatusLabel(job.status)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

async function runPluginAction(
  action: Extract<WorkflowAction, { type: "runPluginOperation" }>,
  chapterText: string,
  characterNames: string[],
): Promise<string> {
  if (action.pluginId === "hello-names") {
    if (!chapterText.trim()) {
      throw new Error(t("plugin.needChapterThrow"));
    }
    const names = splitNames(joinList(characterNames));
    if (names.length === 0) {
      throw new Error(t("plugin.needCharacters"));
    }
    const output = await libraryApi.runPluginOperation(action.pluginId, action.operation, {
      selection: chapterText,
      names,
      ...(typeof action.input === "object" && action.input ? action.input : {}),
    });
    return formatPluginResult(
      {
        id: action.pluginId,
        name: pluginDisplayName({ id: action.pluginId, name: t("plugin.helloNames") }),
        version: "0.1.0",
        runtime: "wasm",
        operations: [action.operation],
      },
      output,
    );
  }
  const output = await libraryApi.runPluginOperation(
    action.pluginId,
    action.operation,
    action.input ?? {},
  );
  return formatPluginResult(
    {
      id: action.pluginId,
      name: pluginDisplayName({ id: action.pluginId, name: action.pluginId }),
      version: "0.1.0",
      runtime: "builtin",
      operations: [action.operation],
    },
    output,
  );
}
