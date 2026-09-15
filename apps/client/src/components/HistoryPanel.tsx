import { useEffect, useMemo, useState } from "react";
import { History, RotateCcw, X } from "lucide-react";
import { libraryApi } from "../api";
import { DiffLine, RevisionDiff, RevisionSummary } from "../types";
import { logger } from "../logger";
import { collatorLocale, formatDiffSummary, useI18n, type Locale } from "../i18n";

interface HistoryPanelProps {
  open: boolean;
  chapterId: string | null;
  chapterTitle?: string | undefined;
  currentRevision: number;
  onClose: () => void;
  onRestore: (revision: number) => Promise<void>;
}

export function HistoryPanel({
  open,
  chapterId,
  chapterTitle,
  currentRevision,
  onClose,
  onRestore,
}: HistoryPanelProps) {
  const { t, locale } = useI18n();
  const [revisions, setRevisions] = useState<RevisionSummary[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [base, setBase] = useState<number>(0);
  const [diff, setDiff] = useState<RevisionDiff | null>(null);
  const [loading, setLoading] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open || !chapterId) {
      setRevisions([]);
      setDiff(null);
      setSelected(null);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    libraryApi
      .listChapterRevisions(chapterId)
      .then((items) => {
        if (cancelled) return;
        setRevisions(items);
        const latest = items[0]?.revision ?? null;
        setSelected(latest);
        const previous = items[1]?.revision ?? 0;
        setBase(previous);
      })
      .catch((err) => {
        if (cancelled) return;
        logger.warn("加载修订历史失败", { error: String(err) });
        setError(String(err));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open, chapterId, currentRevision]);

  useEffect(() => {
    if (!open || !chapterId || selected === null) {
      setDiff(null);
      return;
    }
    let cancelled = false;
    libraryApi
      .diffChapterRevisions(chapterId, base, selected)
      .then((result) => {
        if (!cancelled) setDiff(result);
      })
      .catch((err) => {
        if (!cancelled) {
          logger.warn("对比修订失败", { error: String(err) });
          setError(String(err));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [open, chapterId, base, selected]);

  const selectedRecord = useMemo(
    () => revisions.find((item) => item.revision === selected) ?? null,
    [revisions, selected],
  );

  if (!open) return null;

  async function handleRestore() {
    if (selected === null) return;
    setRestoring(true);
    try {
      await onRestore(selected);
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setRestoring(false);
    }
  }

  return (
    <div className="history-overlay" onClick={onClose}>
      <div className="history-panel" onClick={(event) => event.stopPropagation()}>
        <div className="history-header">
          <div>
            <h3>
              <History size={14} />
              {t("history.title")}
            </h3>
            <p className="history-subtitle">
              {selected !== null
                ? t("history.compare", {
                    title: chapterTitle ?? t("library.noChapter"),
                    base,
                    selected,
                  })
                : (chapterTitle ?? t("library.noChapter"))}
            </p>
          </div>
          <button className="icon-button" onClick={onClose} title={t("common.close")}>
            <X size={14} />
          </button>
        </div>

        {error && <div className="history-error">{error}</div>}

        <div className="history-body">
          <aside className="history-list">
            {loading && <div className="history-empty">{t("history.loading")}</div>}
            {!loading && revisions.length === 0 && (
              <div className="history-empty">{t("history.empty")}</div>
            )}
            {revisions.map((item, index) => (
              <button
                key={item.revision}
                className={`history-item ${item.revision === selected ? "active" : ""} ${
                  item.revision === base ? "base" : ""
                }`}
                onClick={() => {
                  setSelected(item.revision);
                  const previous = revisions[index + 1]?.revision ?? 0;
                  setBase(previous);
                }}
              >
                <span className="history-rev">R{item.revision}</span>
                <span className="history-meta">
                  {formatTime(item.createdAt, locale)} · {t("history.chars", { count: item.charCount })}
                </span>
                <span className="history-preview">{item.preview || t("history.emptyPreview")}</span>
              </button>
            ))}
          </aside>

          <section className="history-diff">
            <div className="history-diff-toolbar">
              <span>
                {diff
                  ? formatDiffSummary(diff.insertedChars, diff.deletedChars, diff.lines.length === 0)
                  : t("history.pickVersion")}
              </span>
              <button
                className="btn"
                disabled={selected === null || selected === currentRevision || restoring}
                onClick={() => void handleRestore()}
              >
                <RotateCcw size={12} />
                {restoring ? t("history.restoring") : t("history.restore")}
              </button>
            </div>
            <div className="history-diff-scroll">
              {diff?.lines.length ? (
                diff.lines.map((line, index) => <DiffRow key={index} line={line} />)
              ) : (
                <div className="history-empty">
                  {selectedRecord ? t("history.identical") : t("history.needSave")}
                </div>
              )}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}

function DiffRow({ line }: { line: DiffLine }) {
  const sign = line.tag === "insert" ? "+" : line.tag === "delete" ? "-" : " ";
  const spans = line.spans?.length ? line.spans : [{ tag: line.tag, text: line.text }];
  return (
    <pre className={`diff-line diff-${line.tag}`}>
      <span className="diff-sign">{sign}</span>
      <span className="diff-text">
        {spans.map((span, index) => (
          <span key={index} className={`diff-span diff-span-${span.tag}`}>
            {span.text || " "}
          </span>
        ))}
      </span>
    </pre>
  );
}

function formatTime(value: string, locale: Locale): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString(collatorLocale(locale), {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}
