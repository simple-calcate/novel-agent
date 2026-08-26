import { X } from "lucide-react";
import { PluginSummary } from "../types";

interface Props {
  open: boolean;
  plugins: PluginSummary[];
  onClose: () => void;
}

export function PluginModal({ open, plugins, onClose }: Props) {
  if (!open) return null;
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
            打包插件走内置执行器。桌面在清单带 <code>wasmBase64</code> 时于 wasmi
            沙箱运行（无 WASI、无文件系统）。Android 仍只支持声明式工作流与内置操作。
          </p>
          <ul className="plugin-list">
            {plugins.map((plugin) => (
              <li key={plugin.id}>
                <strong>{plugin.name}</strong>
                <span>
                  {plugin.id} · {plugin.version} · {plugin.runtime}
                </span>
                <em>{plugin.operations.join("、") || "无操作"}</em>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </div>
  );
}
