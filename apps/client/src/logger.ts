type LogLevel = "debug" | "info" | "warn" | "error";

interface LogEntry {
  timestamp: string;
  level: LogLevel;
  message: string;
  data?: Record<string, unknown> | undefined;
}

const MAX_LOG_ENTRIES = 500;
const logs: LogEntry[] = [];
const listeners: Array<(entry: LogEntry) => void> = [];

function formatMessage(level: LogLevel, message: string, data?: Record<string, unknown>) {
  const timestamp = new Date().toISOString();
  const entry: LogEntry = { timestamp, level, message, data };
  logs.push(entry);
  if (logs.length > MAX_LOG_ENTRIES) {
    logs.shift();
  }
  listeners.forEach((fn) => fn(entry));

  const prefix = `[${timestamp}] [${level.toUpperCase()}]`;
  if (data) {
    console.log(prefix, message, data);
  } else {
    console.log(prefix, message);
  }
}

export const logger = {
  debug: (message: string, data?: Record<string, unknown>) => formatMessage("debug", message, data),
  info: (message: string, data?: Record<string, unknown>) => formatMessage("info", message, data),
  warn: (message: string, data?: Record<string, unknown>) => formatMessage("warn", message, data),
  error: (message: string, data?: Record<string, unknown>) => formatMessage("error", message, data),

  getLogs: () => [...logs],
  subscribe: (fn: (entry: LogEntry) => void) => {
    listeners.push(fn);
    return () => {
      const idx = listeners.indexOf(fn);
      if (idx >= 0) listeners.splice(idx, 1);
    };
  },
};
