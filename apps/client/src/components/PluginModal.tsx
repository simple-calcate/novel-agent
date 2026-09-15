import { useEffect, useMemo, useState } from "react";
import { Play, X } from "lucide-react";
import { PluginSummary } from "../types";
import { libraryApi } from "../api";
import { formatPluginResult, pluginDisplayName, pluginIsRunnable, splitNames } from "../plugins/format";
import { joinList, useI18n } from "../i18n";

interface Props {
  open: boolean;
  plugins: PluginSummary[];
  chapterText: string;
  characterNames: string[];
  onClose: () => void;
}

export function PluginModal({ open, plugins, chapterText, characterNames, onClose }: Props) {
  const { t, locale } = useI18n();
  const defaultNames = useMemo(() => joinList(characterNames), [characterNames, locale]);
  const [nameInput, setNameInput] = useState(defaultNames);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const names = splitNames(nameInput);

  useEffect(() => {
    if (!open) return;
    setNameInput(defaultNames);
    setResult(null);
    setError(null);
    setBusyId(null);
  }, [open, defaultNames]);

  if (!open) return null;

  async function run(plugin: PluginSummary) {
    const operation = plugin.operations[0];
    if (!operation) {
      setError(t("plugin.noOperation"));
      return;
    }
    if (plugin.id === "hello-names") {
      if (!chapterText.trim()) {
        setError(t("plugin.noChapter"));
        return;
      }
      if (names.length === 0) {
        setError(t("plugin.needNames"));
        return;
      }
    }
    setBusyId(plugin.id);
    setError(null);
    setResult(null);
    try {
      const input =
        plugin.id === "hello-names" ? { selection: chapterText, names } : {};
      const output = await libraryApi.runPluginOperation(plugin.id, operation, input);
      setResult(formatPluginResult(plugin, output));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusyId(null);
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(event) => event.stopPropagation()}>
        <div className="modal-header">
          <h2>{t("plugin.title")}</h2>
          <button className="icon-button" onClick={onClose} title={t("common.close")}>
            <X size={16} />
          </button>
        </div>
        <div className="modal-body">
          <p className="panel-muted">{t("plugin.lead")}</p>
          <label className="plugin-names">
            <span>{t("plugin.names")}</span>
            <input
              value={nameInput}
              onChange={(event) => setNameInput(event.target.value)}
              placeholder={defaultNames || t("plugin.namesPlaceholder")}
            />
          </label>
          {!chapterText.trim() && <p className="panel-muted">{t("plugin.noChapter")}</p>}
          <ul className="plugin-list">
            {plugins.map((plugin) => {
              const runnable = pluginIsRunnable(plugin);
              const busy = busyId === plugin.id;
              return (
                <li key={plugin.id}>
                  <div className="plugin-row">
                    <div>
                      <strong>
                        {pluginDisplayName(plugin)}
                        <span className={`plugin-badge ${runnable ? "wasm" : "builtin"}`}>
                          {runnable ? t("plugin.runnable") : t("plugin.placeholder")}
                        </span>
                      </strong>
                      <span>
                        {plugin.id} · {plugin.version} · {plugin.runtime}
                      </span>
                      <em>{joinList(plugin.operations) || t("plugin.noOps")}</em>
                    </div>
                    <button
                      className="mini-button"
                      disabled={busyId !== null || plugin.operations.length === 0}
                      onClick={() => void run(plugin)}
                      title={runnable ? t("plugin.runOnChapter") : t("plugin.viewReceipt")}
                    >
                      <Play size={12} />
                      {busy ? t("common.running") : t("common.run")}
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
          {error && <p className="plugin-error">{error}</p>}
          {result && <pre className="plugin-result">{result}</pre>}
        </div>
      </div>
    </div>
  );
}
