import { Plus, X } from "lucide-react";
import { useState } from "react";
import { StoryEntry, StoryEntryKind } from "../types";

const KIND_ORDER: StoryEntryKind[] = ["character", "setting", "foreshadow"];

const KIND_LABELS: Record<StoryEntryKind, string> = {
  character: "人物",
  setting: "设定",
  foreshadow: "伏笔",
};

interface Props {
  disabled: boolean;
  busy: boolean;
  error: string | null;
  entries: StoryEntry[];
  onCreate: (kind: StoryEntryKind, title: string, summary: string) => void;
  onDelete: (entry: StoryEntry) => void;
}

export function StructurePanel({ disabled, busy, error, entries, onCreate, onDelete }: Props) {
  const [kind, setKind] = useState<StoryEntryKind>("character");
  const [title, setTitle] = useState("");
  const [summary, setSummary] = useState("");

  const submit = () => {
    if (!title.trim()) return;
    onCreate(kind, title.trim(), summary.trim());
    setTitle("");
    setSummary("");
  };

  return (
    <div className="panel-content">
      <h3>结构{entries.length > 0 ? `（${entries.length}）` : ""}</h3>
      <p className="canon-lead">
        预先写好人物、设定和伏笔。写作时按名称、别名和设定关键词匹配当前段落；本地没命中时再用这段里的词去说明里检索。
      </p>
      {error && <div className="tree-empty">{error}</div>}

      {entries.length === 0 && (
        <div className="empty-state">还没有结构。先添加人物、设定或伏笔。</div>
      )}
      {KIND_ORDER.map((group) => {
        const items = entries.filter((entry) => entry.kind === group);
        if (items.length === 0) return null;
        return (
          <section key={group} className="structure-group">
            <h4 className="structure-group-title">{KIND_LABELS[group]}</h4>
            {items.map((entry) => (
              <div key={entry.id} className="context-card canon-card">
                <div className="context-card-title">
                  {entry.title}
                  {entry.aliases?.length > 0 && (
                    <span className="canon-kind">{entry.aliases.join("、")}</span>
                  )}
                  <button className="icon-button" title="删除" onClick={() => onDelete(entry)}>
                    <X size={12} />
                  </button>
                </div>
                {entry.summary && <p>{entry.summary}</p>}
              </div>
            ))}
          </section>
        );
      })}

      <div className="structure-form">
        <div className="structure-kinds">
          {KIND_ORDER.map((item) => (
            <button
              key={item}
              className={kind === item ? "mini-button active" : "mini-button"}
              onClick={() => setKind(item)}
              type="button"
            >
              {KIND_LABELS[item]}
            </button>
          ))}
        </div>
        <input
          className="structure-input"
          placeholder={
            kind === "character"
              ? "人名，可写别名：林晚、雾儿"
              : kind === "foreshadow"
                ? "伏笔名称，例如：雾中灯塔"
                : "设定名称，例如：雾港"
          }
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          disabled={disabled || busy}
        />
        <textarea
          className="structure-input"
          placeholder="补充说明（可选）"
          rows={3}
          value={summary}
          onChange={(event) => setSummary(event.target.value)}
          disabled={disabled || busy}
        />
        <button className="mini-button" type="button" disabled={disabled || busy || !title.trim()} onClick={submit}>
          <Plus size={12} />
          添加
        </button>
      </div>
    </div>
  );
}
