import { Plus, X } from "lucide-react";
import { useState } from "react";
import { StoryEntry, StoryEntryKind } from "../types";
import { joinList, useI18n } from "../i18n";

const KIND_ORDER: StoryEntryKind[] = ["character", "setting", "foreshadow"];

interface Props {
  disabled: boolean;
  busy: boolean;
  error: string | null;
  entries: StoryEntry[];
  onCreate: (kind: StoryEntryKind, title: string, summary: string) => void;
  onDelete: (entry: StoryEntry) => void;
}

export function StructurePanel({ disabled, busy, error, entries, onCreate, onDelete }: Props) {
  const { t } = useI18n();
  const [kind, setKind] = useState<StoryEntryKind>("character");
  const [title, setTitle] = useState("");
  const [summary, setSummary] = useState("");
  const kindLabels: Record<StoryEntryKind, string> = {
    character: t("rail.character"),
    setting: t("rail.setting"),
    foreshadow: t("rail.foreshadow"),
  };

  const submit = () => {
    if (!title.trim()) return;
    onCreate(kind, title.trim(), summary.trim());
    setTitle("");
    setSummary("");
  };

  return (
    <div className="panel-content">
      <h3>{entries.length > 0 ? t("structure.headingCount", { count: entries.length }) : t("structure.heading")}</h3>
      <p className="canon-lead">{t("structure.lead")}</p>
      {error && <div className="tree-empty">{error}</div>}

      {entries.length === 0 && <div className="empty-state">{t("structure.empty")}</div>}
      {KIND_ORDER.map((group) => {
        const items = entries.filter((entry) => entry.kind === group);
        if (items.length === 0) return null;
        return (
          <section key={group} className="structure-group">
            <h4 className="structure-group-title">{kindLabels[group]}</h4>
            {items.map((entry) => (
              <div key={entry.id} className="context-card canon-card">
                <div className="context-card-title">
                  {entry.title}
                  {entry.aliases?.length > 0 && (
                    <span className="canon-kind">{joinList(entry.aliases)}</span>
                  )}
                  <button className="icon-button" title={t("structure.delete")} onClick={() => onDelete(entry)}>
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
              {kindLabels[item]}
            </button>
          ))}
        </div>
        <input
          className="structure-input"
          placeholder={
            kind === "character"
              ? t("structure.characterPlaceholder")
              : kind === "foreshadow"
                ? t("structure.foreshadowPlaceholder")
                : t("structure.settingPlaceholder")
          }
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          disabled={disabled || busy}
        />
        <textarea
          className="structure-input"
          placeholder={t("structure.summaryPlaceholder")}
          rows={3}
          value={summary}
          onChange={(event) => setSummary(event.target.value)}
          disabled={disabled || busy}
        />
        <button className="mini-button" type="button" disabled={disabled || busy || !title.trim()} onClick={submit}>
          <Plus size={12} />
          {t("common.add")}
        </button>
      </div>
    </div>
  );
}
