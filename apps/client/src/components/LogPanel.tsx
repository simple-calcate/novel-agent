import { useEffect, useState } from "react";
import { X, Trash2, Download } from "lucide-react";
import { logger } from "../logger";

interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
  data?: Record<string, unknown> | undefined;
}

export function LogPanel({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [logs, setLogs] = useState<LogEntry[]>([]);

  useEffect(() => {
    setLogs(logger.getLogs());
    const unsubscribe = logger.subscribe((entry) => {
      setLogs((current) => [...current.slice(-499), entry]);
    });
    return unsubscribe;
  }, []);

  if (!open) return null;

  const clearLogs = () => {
    logger.getLogs().length = 0;
    setLogs([]);
  };

  const exportLogs = () => {
    const text = logs
      .map((l) => `[${l.timestamp}] [${l.level.toUpperCase()}] ${l.message} ${l.data ? JSON.stringify(l.data) : ""}`)
      .join("\n");
    const blob = new Blob([text], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `novel-agent-logs-${new Date().toISOString().slice(0, 19)}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="log-panel-overlay" onClick={onClose}>
      <div className="log-panel" onClick={(e) => e.stopPropagation()}>
        <div className="log-panel-header">
          <h3>应用日志</h3>
          <div className="log-panel-actions">
            <button className="mini-button" onClick={exportLogs} title="导出日志">
              <Download size={12} />
            </button>
            <button className="mini-button" onClick={clearLogs} title="清空日志">
              <Trash2 size={12} />
            </button>
            <button className="icon-button" onClick={onClose}>
              <X size={14} />
            </button>
          </div>
        </div>
        <div className="log-panel-content">
          {logs.length === 0 ? (
            <div className="log-empty">暂无日志</div>
          ) : (
            logs.map((entry, i) => (
              <div key={i} className={`log-entry log-${entry.level}`}>
                <span className="log-time">{entry.timestamp.slice(11, 19)}</span>
                <span className="log-level">[{entry.level.toUpperCase()}]</span>
                <span className="log-message">{entry.message}</span>
                {entry.data && (
                  <pre className="log-data">{JSON.stringify(entry.data, null, 2)}</pre>
                )}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
