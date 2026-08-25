import { FormEvent, useEffect, useState } from "react";
import { X } from "lucide-react";

interface Props {
  open: boolean;
  title: string;
  label: string;
  placeholder: string;
  confirmLabel?: string;
  initialValue?: string;
  onClose: () => void;
  onSubmit: (value: string) => void | Promise<void>;
}

export function CreateDialog({
  open,
  title,
  label,
  placeholder,
  confirmLabel = "创建",
  initialValue = "",
  onClose,
  onSubmit,
}: Props) {
  const [value, setValue] = useState(initialValue);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setValue(initialValue);
      setError(null);
      setBusy(false);
    }
  }, [open, initialValue]);

  if (!open) return null;

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    const trimmed = value.trim();
    if (!trimmed) {
      setError("名称不能为空");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await onSubmit(trimmed);
      setValue("");
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <form className="modal" onClick={(event) => event.stopPropagation()} onSubmit={handleSubmit}>
        <div className="modal-header">
          <h2>{title}</h2>
          <button type="button" className="icon-button" onClick={onClose}>
            <X size={16} />
          </button>
        </div>
        <div className="modal-body">
          <label className="field">
            <span className="field-label">{label}</span>
            <input
              className="text-input"
              autoFocus
              value={value}
              placeholder={placeholder}
              onChange={(event) => setValue(event.target.value)}
            />
          </label>
          {error && <p className="field-error">{error}</p>}
        </div>
        <div className="modal-footer">
          <button type="button" className="btn" onClick={onClose}>
            取消
          </button>
          <button type="submit" className="btn primary" disabled={busy}>
            {busy ? "请稍候…" : confirmLabel}
          </button>
        </div>
      </form>
    </div>
  );
}
