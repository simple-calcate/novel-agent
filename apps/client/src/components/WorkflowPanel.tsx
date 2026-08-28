import { FileDown, Play, Workflow } from "lucide-react";
import { useEffect, useState } from "react";
import {
  actionToToolId,
  bundledWorkflowTemplates,
} from "@novel-agent/workflow-builder";
import { libraryApi } from "../api";

interface Props {
  jobs: Array<{ id: string; label: string; status: string }>;
  queueReady: boolean;
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

export function WorkflowPanel({ jobs, queueReady, onRun }: Props) {
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

  return (
    <div className="panel-content">
      <div className="panel-heading">
        <h3>工作流</h3>
      </div>
      <p className="panel-muted">
        模板来自 MIT 包 <code>@novel-agent/workflow-builder</code>
        。可视化编辑器还没有；点播放会按顺序入队模板里的动作。
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
      {message && <p className="panel-muted outbox-journal-note">{message}</p>}

      <div className="workflow-list">
        {templates.map((template) => (
          <div key={template.id} className="workflow-item">
            <div className="workflow-icon">
              <Workflow size={14} />
            </div>
            <div className="workflow-body">
              <div className="workflow-name">{template.name}</div>
              <div className="workflow-trigger">{template.trigger}</div>
            </div>
            <button
              className="mini-button"
              onClick={() => {
                for (const action of template.actions) {
                  onRun(actionToToolId(action), template.name);
                }
              }}
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
