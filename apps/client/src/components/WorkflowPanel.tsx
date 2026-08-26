import { useEffect, useState } from "react";
import { FileDown, Play, Plus, Workflow } from "lucide-react";
import { libraryApi } from "../api";

interface Props {
  jobs: Array<{ id: string; label: string; status: string }>;
  queueReady: boolean;
  onRun: (operation: string, label: string) => void;
}

const templates: Array<{ label: string; trigger: string; operation: string }> = [
  { label: "停笔后自动保存并刷新索引", trigger: "editor.idle", operation: "index.rebuild" },
  { label: "新章节生成大纲草稿", trigger: "chapter.created", operation: "agent.continuation" },
  { label: "新段落超过 200 字时检查设定", trigger: "paragraph.created", operation: "continuity.check" },
  { label: "保存后运行连续性检查", trigger: "document.saved", operation: "continuity.check" },
];

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
        <button className="mini-button">
          <Plus size={12} />
          新建
        </button>
      </div>

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
          <div key={template.label} className="workflow-item">
            <div className="workflow-icon">
              <Workflow size={14} />
            </div>
            <div className="workflow-body">
              <div className="workflow-name">{template.label}</div>
              <div className="workflow-trigger">{template.trigger}</div>
            </div>
            <button
              className="mini-button"
              onClick={() => onRun(template.operation, template.label)}
              title="立即运行"
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
