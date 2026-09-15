import type { MouseEvent } from "react";
import { Pencil, Trash2, ChevronUp, ChevronDown } from "lucide-react";
import { useI18n } from "../i18n";

interface Props {
  disableUp?: boolean;
  disableDown?: boolean;
  deleteTitle?: string;
  onRename: () => void;
  onDelete: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
}

export function TreeItemActions({
  disableUp,
  disableDown,
  deleteTitle,
  onRename,
  onDelete,
  onMoveUp,
  onMoveDown,
}: Props) {
  const { t } = useI18n();
  const stop = (event: MouseEvent, action: () => void) => {
    event.preventDefault();
    event.stopPropagation();
    action();
  };

  return (
    <span className="tree-actions" onClick={(event) => event.stopPropagation()}>
      <button type="button" title={t("common.moveUp")} disabled={disableUp} onClick={(event) => stop(event, onMoveUp)}>
        <ChevronUp size={12} />
      </button>
      <button
        type="button"
        title={t("common.moveDown")}
        disabled={disableDown}
        onClick={(event) => stop(event, onMoveDown)}
      >
        <ChevronDown size={12} />
      </button>
      <button type="button" title={t("common.rename")} onClick={(event) => stop(event, onRename)}>
        <Pencil size={12} />
      </button>
      <button
        type="button"
        title={deleteTitle ?? t("common.delete")}
        onClick={(event) => stop(event, onDelete)}
      >
        <Trash2 size={12} />
      </button>
    </span>
  );
}

interface ConfirmProps {
  open: boolean;
  title: string;
  body: string;
  confirmLabel?: string;
  onClose: () => void;
  onConfirm: () => void | Promise<void>;
}

export function ConfirmDialog({
  open,
  title,
  body,
  confirmLabel,
  onClose,
  onConfirm,
}: ConfirmProps) {
  const { t } = useI18n();
  if (!open) return null;
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(event) => event.stopPropagation()}>
        <div className="modal-header">
          <h2>{title}</h2>
        </div>
        <div className="modal-body">
          <p>{body}</p>
        </div>
        <div className="modal-footer">
          <button type="button" className="btn" onClick={onClose}>
            {t("common.cancel")}
          </button>
          <button
            type="button"
            className="btn primary"
            onClick={() => {
              void Promise.resolve(onConfirm())
                .then(onClose)
                .catch(() => undefined);
            }}
          >
            {confirmLabel ?? t("common.delete")}
          </button>
        </div>
      </div>
    </div>
  );
}
