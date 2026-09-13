import { Pencil, Plus, X } from "lucide-react";
import { useEffect, useState } from "react";
import { joinTitleAndAliases } from "../structure/match";
import { StoryEntry, StoryEntryKind } from "../types";
import { ConfirmDialog } from "./LibraryActions";

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
  focusId: string | null;
  focusNonce: number;
  onCreate: (kind: StoryEntryKind, title: string, summary: string) => void;
  onUpdate: (entry: StoryEntry, title: string, summary: string) => void;
  onDelete: (entry: StoryEntry) => void;
}

export function StructurePanel({
  disabled,
  busy,
  error,
  entries,
  focusId,
  focusNonce,
  onCreate,
  onUpdate,
  onDelete,
}: Props) {
  const [kind, setKind] = useState<StoryEntryKind>("character");
  const [title, setTitle] = useState("");
  const [summary, setSummary] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draftTitle, setDraftTitle] = useState("");
  const [draftSummary, setDraftSummary] = useState("");
  const [pendingDelete, setPendingDelete] = useState<StoryEntry | null>(null);
  const [seenFocus, setSeenFocus] = useState(0);

  if (focusNonce !== seenFocus && focusId) {
    const entry = entries.find((item) => item.id === focusId);
    setSeenFocus(focusNonce);
    if (entry) {
      setEditingId(entry.id);
      setDraftTitle(joinTitleAndAliases(entry.title, entry.aliases));
      setDraftSummary(entry.summary ?? "");
    }
  }

  useEffect(() => {
    if (!focusId || focusNonce === 0) return;
    document.getElementById(`structure-${focusId}`)?.scrollIntoView({ block: "nearest" });
  }, [focusId, focusNonce]);

  const submit = () => {
    if (!title.trim()) return;
    onCreate(kind, title.trim(), summary.trim());
    setTitle("");
    setSummary("");
  };

  const beginEdit = (entry: StoryEntry) => {
    setEditingId(entry.id);
    setDraftTitle(joinTitleAndAliases(entry.title, entry.aliases));
    setDraftSummary(entry.summary ?? "");
  };

  const saveEdit = (entry: StoryEntry) => {
    if (!draftTitle.trim()) return;
    onUpdate(entry, draftTitle.trim(), draftSummary.trim());
    setEditingId(null);
  };

  return (
    <div className="panel-content">
      <h3>结构{entries.length > 0 ? `（${entries.length}）` : ""}</h3>
      <p className="canon-lead">
        预先写好人物、设定和伏笔。点卡片可改名称、别名和说明；写作时按这些匹配当前段落。
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
            {items.map((entry) => {
              const editing = editingId === entry.id;
              const focused = focusId === entry.id;
              return (
                <div
                  key={entry.id}
                  id={`structure-${entry.id}`}
                  className={`context-card canon-card ${focused ? "focused" : ""}`}
                  onClick={() => {
                    if (!editing) beginEdit(entry);
                  }}
                >
                  {editing ? (
                    <div className="structure-edit" onClick={(event) => event.stopPropagation()}>
                      <input
                        className="structure-input"
                        value={draftTitle}
                        onChange={(event) => setDraftTitle(event.target.value)}
                        onKeyDown={(event) => {
                          if (event.key === "Enter") {
                            event.preventDefault();
                            saveEdit(entry);
                          }
                          if (event.key === "Escape") setEditingId(null);
                        }}
                        disabled={disabled || busy}
                        autoFocus
                      />
                      <textarea
                        className="structure-input"
                        rows={3}
                        value={draftSummary}
                        onChange={(event) => setDraftSummary(event.target.value)}
                        disabled={disabled || busy}
                      />
                      <div className="structure-edit-actions">
                        <button
                          className="mini-button"
                          type="button"
                          disabled={disabled || busy || !draftTitle.trim()}
                          onClick={() => saveEdit(entry)}
                        >
                          保存
                        </button>
                        <button
                          className="mini-button"
                          type="button"
                          onClick={() => setEditingId(null)}
                        >
                          取消
                        </button>
                      </div>
                    </div>
                  ) : (
                    <>
                      <div className="context-card-title">
                        {entry.title}
                        {entry.aliases?.length > 0 && (
                          <span className="canon-kind">{entry.aliases.join("、")}</span>
                        )}
                        <span
                          className="structure-card-actions"
                          onClick={(event) => event.stopPropagation()}
                        >
                          <button
                            className="icon-button"
                            title="编辑"
                            type="button"
                            onClick={() => beginEdit(entry)}
                          >
                            <Pencil size={12} />
                          </button>
                          <button
                            className="icon-button"
                            type="button"
                            title={`删除「${entry.title}」`}
                            aria-label={`删除${KIND_LABELS[group]}「${entry.title}」`}
                            onClick={() => setPendingDelete(entry)}
                          >
                            <X size={12} />
                          </button>
                        </span>
                      </div>
                      {entry.summary && <p>{entry.summary}</p>}
                    </>
                  )}
                </div>
              );
            })}
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
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              submit();
            }
          }}
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

      <ConfirmDialog
        open={pendingDelete !== null}
        title="删除结构"
        body={`确定删除「${pendingDelete?.title ?? ""}」？预选条不再匹配这条。`}
        onClose={() => setPendingDelete(null)}
        onConfirm={() => {
          if (pendingDelete) onDelete(pendingDelete);
        }}
      />
    </div>
  );
}
