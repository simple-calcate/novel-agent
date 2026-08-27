import { Capability, PluginManifest, PluginOperation } from "./types";

export type { Capability, PluginManifest, PluginOperation };

const ID_PATTERN = /^[a-z0-9][a-z0-9-]*[a-z0-9]$/;
const VERSION_PATTERN = /^\d+\.\d+\.\d+$/;
const PLATFORMS = new Set(["linux", "windows", "android"]);

export class PluginManifestError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "PluginManifestError";
  }
}

/** 编写插件时的入口：校验清单并填上缺省值，返回可序列化的清单。 */
export function definePlugin(input: PluginManifest): PluginManifest {
  const id = input.id?.trim() ?? "";
  if (!ID_PATTERN.test(id)) {
    throw new PluginManifestError(
      "id 必须是小写字母、数字和连字符，至少两字符，例如 my-check",
    );
  }
  const name = input.name?.trim() ?? "";
  if (!name) {
    throw new PluginManifestError("name 不能为空");
  }
  const version = input.version?.trim() ?? "";
  if (!VERSION_PATTERN.test(version)) {
    throw new PluginManifestError("version 必须是 semver，例如 0.1.0");
  }
  const apiVersion = input.apiVersion ?? 1;
  if (apiVersion !== 1) {
    throw new PluginManifestError("目前只支持 apiVersion = 1");
  }
  const platforms = [...new Set(input.platforms ?? [])];
  if (platforms.length === 0) {
    throw new PluginManifestError("至少声明一个平台：linux / windows / android");
  }
  for (const platform of platforms) {
    if (!PLATFORMS.has(platform)) {
      throw new PluginManifestError(`不支持的平台：${platform}`);
    }
  }
  const operations = (input.operations ?? []).map(normalizeOperation);
  if (operations.length === 0) {
    throw new PluginManifestError("至少声明一个 operation");
  }
  const names = new Set<string>();
  for (const operation of operations) {
    if (names.has(operation.name)) {
      throw new PluginManifestError(`重复的 operation：${operation.name}`);
    }
    names.add(operation.name);
  }

  const manifest: PluginManifest = {
    id,
    name,
    version,
    apiVersion,
    platforms,
    operations,
    requestedCapabilities: input.requestedCapabilities ?? [],
  };
  if (input.settingsSchema) {
    manifest.settingsSchema = input.settingsSchema;
  }
  if (input.wasmBase64?.trim()) {
    manifest.wasmBase64 = input.wasmBase64.trim();
  }
  return manifest;
}

export function toManifestJson(manifest: PluginManifest): string {
  return `${JSON.stringify(definePlugin(manifest), null, 2)}\n`;
}

export function capabilityKind(capability: Capability): string {
  return capability.kind;
}

function normalizeOperation(operation: PluginOperation): PluginOperation {
  const name = operation.name?.trim() ?? "";
  if (!name) {
    throw new PluginManifestError("operation.name 不能为空");
  }
  const normalized: PluginOperation = {
    name,
    inputSchema: operation.inputSchema ?? { type: "object" },
    outputSchema: operation.outputSchema ?? { type: "object" },
    triggers: operation.triggers ?? [],
  };
  if (operation.description) {
    normalized.description = operation.description;
  }
  return normalized;
}
