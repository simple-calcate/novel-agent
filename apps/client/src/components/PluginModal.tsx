import { useMemo, useState } from "react";
import { Play, X } from "lucide-react";
import { PluginRunResult, PluginSummary } from "../types";
import { libraryApi } from "../api";

interface Props {
  open: boolean;
  plugins: PluginSummary[];
  chapterText: string;
  characterNames: string[];
  onClose: () => void;
}

export function PluginModal({ open, plugins, chapterText, characterNames, onClose }: Props) {
  const defaultNames = useMemo(() => characterNames.join("、"), [characterNames]);
  const [nameInput, setNameInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const names = splitNames(nameInput || defaultNames);

  if (!open) return null;

  async function run(plugin: PluginSummary) {
    const operation = plugin.operations[0];
    if (!operation) {
      setError("这个插件没有可运行的操作");
      return;
    }
    setBusy(true);
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
      setBusy(false);
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(event) => event.stopPropagation()}>
        <div className="modal-header">
          <h2>已打包插件</h2>
          <button className="icon-button" onClick={onClose} title="关闭">
            <X size={16} />
          </button>
        </div>
        <div className="modal-body">
          <p className="panel-muted">
            「人名点名」会对当前正文统计人名（桌面走 wasmi，浏览器预览走 SDK）。其它打包项仍是内置占位。
            第三方用 MIT 的 <code>@novel-agent/plugin-sdk</code> 写清单，用{" "}
            <code>@novel-agent/plugin-compile</code> 编成 WASM。宿主本身不是开源软件。
          </p>
          <label className="plugin-names">
            <span>要统计的人名</span>
            <input
              value={nameInput || defaultNames}
              onChange={(event) => setNameInput(event.target.value)}
              placeholder={defaultNames || "林默、林晚"}
            />
          </label>
          {!chapterText.trim() && (
            <p className="panel-muted">当前没有打开的章节正文。打开一章或点「打开示例章节」再运行。</p>
          )}
          <ul className="plugin-list">
            {plugins.map((plugin) => (
              <li key={plugin.id}>
                <div className="plugin-row">
                  <div>
                    <strong>{plugin.name}</strong>
                    <span>
                      {plugin.id} · {plugin.version} · {plugin.runtime}
                    </span>
                    <em>{plugin.operations.join("、") || "无操作"}</em>
                  </div>
                  <button
                    className="mini-button"
                    disabled={busy || plugin.operations.length === 0}
                    onClick={() => void run(plugin)}
                    title="对当前正文运行"
                  >
                    <Play size={12} />
                    {busy ? "运行中" : "运行"}
                  </button>
                </div>
              </li>
            ))}
          </ul>
          {error && <p className="plugin-error">{error}</p>}
          {result && <pre className="plugin-result">{result}</pre>}
        </div>
      </div>
    </div>
  );
}

function splitNames(raw: string): string[] {
  return raw
    .split(/[,，、]/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function formatPluginResult(plugin: PluginSummary, result: PluginRunResult): string {
  const counts = (result.output as { counts?: Record<string, number> } | null)?.counts;
  if (plugin.id === "hello-names" && counts) {
    const lines = Object.entries(counts).map(([name, count]) => `${name} × ${count}`);
    return lines.length > 0 ? lines.join("\n") : "没有统计到人名。";
  }
  return JSON.stringify(result.output, null, 2);
}
