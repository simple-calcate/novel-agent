import manifestSchema from "../manifest.schema.json";

export { manifestSchema };

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  apiVersion: number;
  platforms: Array<"linux" | "windows" | "android">;
  operations: PluginOperation[];
  requestedCapabilities: Capability[];
  settingsSchema?: Record<string, unknown>;
  wasmBase64?: string;
}

export interface PluginOperation {
  name: string;
  description?: string;
  inputSchema: Record<string, unknown>;
  outputSchema: Record<string, unknown>;
  triggers: string[];
}

export type Capability =
  | { kind: "readSelection" }
  | { kind: "readChapter"; scope: "current" | "book" | "project" }
  | { kind: "readStoryModel" }
  | { kind: "proposePatch" }
  | { kind: "log" }
  | { kind: "network"; allowlist: string[] }
  | { kind: "model"; provider: string; maxCostMicros: number }
  | { kind: "privateStorage" };

export interface PluginContext {
  pluginId: string;
  capabilities: Capability[];
  log(level: "debug" | "info" | "warn" | "error", message: string): void;
}

export interface PluginResult {
  output: unknown;
  patches?: unknown[];
  logs?: string[];
}
