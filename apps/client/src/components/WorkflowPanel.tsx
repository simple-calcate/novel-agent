import { FileDown, Play, Workflow } from "lucide-react";
import { useEffect, useState } from "react";
import {
  actionToToolId,
  bundledWorkflowTemplates,
  type WorkflowAction,
  type WorkflowDefinition,
} from "@novel-agent/workflow-builder";
import { libraryApi } from "../api";
import { formatPluginResult, splitNames } from "../plugins/format";
import { actionLabel } from "../workflow/labels";

interface Props {
  jobs: Array<{ id: string; label: string; status: string }>;
  queueReady: boolean;
  chapterText: string;
  characterNames: string[];
  onRun: (operation: string, label: string) => void;
}

const templates = bundledWorkflowTemplates();

const STATUS_LABELS: Record<string, string> = {
  pending: "排队中",
  blocked: "阻塞",
  running: "执行中",
  succeeded: "完成",
  failed: "失败",
  cancelled: "已取消",
  deadLetter: "死信",
};

export function WorkflowPanel({
  jobs,
  queueReady,
  chapterText,
  characterNames,
  onRun,
}: Props) {
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
      setMessage(`${result.written} 条已写出${location}。${result.note}`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "写出失败");
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
          onRun(actionToToolId(action), template.name);
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
        <h3>工作流</h3>
      </div>
      <p className="panel-muted">
        模板来自 MIT 包 <code>@novel-agent/workflow-builder</code>
        。可视化编辑器还没有。点播放：队列类动作会入队；「点名当前章」立刻对人名点名。
      </p>

      <div className="outbox-journal">
        <div className="outbox-journal-copy">
          <strong>待同步 {pending} 条</strong>
          <p>本机把变更写成 JSONL，不是设备间同步。</p>
        </div>
        <button className="mini-button" onClick={() => void flushJournal()} disabled={busy}>
          <FileDown size={12} />
          {busy ? "写出中" : "写出 journal"}
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
              <div className="workflow-name">{template.name}</div>
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
              title="立即运行模板中的动作"
            >
              <Play size={12} />
            </button>
          </div>
        ))}
      </div>

      <h3 className="jobs-heading">任务队列</h3>
      <div className="job-list">
        {!queueReady && <div className="empty-state">队列后端未连接</div>}
        {queueReady && jobs.length === 0 && <div className="empty-state">暂无任务</div>}
        {jobs.map((job) => (
          <div key={job.id} className="job-item">
            <span className={`job-status ${job.status === "succeeded" ? "done" : ""} ${job.status}`} />
            <span className="job-label">{job.label}</span>
            <span className="job-state">{STATUS_LABELS[job.status] ?? job.status}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function triggerLabel(trigger: string): string {
  switch (trigger) {
    case "editor.idle":
      return "停笔后";
    case "chapter.created":
      return "新章节";
    case "paragraph.created":
      return "新段落";
    case "document.saved":
      return "保存后";
    case "manual":
      return "手动";
    default:
      return trigger;
  }
}

async function runPluginAction(
  action: Extract<WorkflowAction, { type: "runPluginOperation" }>,
  chapterText: string,
  characterNames: string[],
): Promise<string> {
  if (action.pluginId === "hello-names") {
    if (!chapterText.trim()) {
      throw new Error("先打开一章，或点「打开示例章节」。");
    }
    const names = splitNames(characterNames.join("、"));
    if (names.length === 0) {
      throw new Error("先在结构里加人物，或打开示例章节（会预置林默）。");
    }
    const output = await libraryApi.runPluginOperation(action.pluginId, action.operation, {
      selection: chapterText,
      names,
      ...(typeof action.input === "object" && action.input ? action.input : {}),
    });
    return formatPluginResult(
      {
        id: action.pluginId,
        name: "人名点名",
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
      name: action.pluginId,
      version: "0.1.0",
      runtime: "builtin",
      operations: [action.operation],
    },
    output,
  );
}
