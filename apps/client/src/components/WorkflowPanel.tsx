import { Play, Plus, Workflow } from "lucide-react";

interface Props {
  jobs: Array<{ id: string; label: string; status: string }>;
  onRun: (label: string) => void;
}

const templates = [
  { label: "停笔后自动保存并刷新索引", trigger: "editor.idle" },
  { label: "新章节生成大纲草稿", trigger: "chapter.created" },
  { label: "新段落超过 200 字时检查设定", trigger: "paragraph.created" },
  { label: "保存后运行连续性检查", trigger: "document.saved" },
];

export function WorkflowPanel({ jobs, onRun }: Props) {
  return (
    <div className="panel-content">
      <div className="panel-heading">
        <h3>工作流</h3>
        <button className="mini-button">
          <Plus size={12} />
          新建
        </button>
      </div>

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
              onClick={() => onRun(template.label)}
              title="立即运行"
            >
              <Play size={12} />
            </button>
          </div>
        ))}
      </div>

      <h3 className="jobs-heading">任务队列</h3>
      <div className="job-list">
        {jobs.length === 0 && <div className="empty-state">暂无任务</div>}
        {jobs.map((job) => (
          <div key={job.id} className="job-item">
            <span className={`job-status ${job.status}`} />
            <span className="job-label">{job.label}</span>
            <span className="job-state">{job.status}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
