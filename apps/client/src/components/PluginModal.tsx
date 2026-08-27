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
            打包插件走内置执行器。第三方用 MIT 的 <code>@novel-agent/plugin-sdk</code> 写清单，
            用 <code>@novel-agent/plugin-compile</code> 把 AssemblyScript guest 编成 WASM，写入清单的{" "}
            <code>wasmBase64</code>。桌面 wasmi 沙箱无 WASI、无文件系统。Android
            仍只支持声明式工作流与内置操作。宿主本身不是开源软件。
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
