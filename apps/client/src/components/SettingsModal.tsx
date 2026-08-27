import { useEffect, useState } from "react";
import { X, Key, Cpu, Globe, Server } from "lucide-react";

import { ModelConfig } from "../types";

export type { ModelConfig };

interface Props {
  open: boolean;
  onClose: () => void;
  initialConfig: ModelConfig | null;
  onSave: (config: ModelConfig) => void;
}

const providerPresets: Record<string, { label: string; baseUrl: string; models: string[] }> = {
  openai: {
    label: "OpenAI",
    baseUrl: "https://api.openai.com/v1",
    models: ["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna", "gpt-5.5"],
  },
  anthropic: {
    label: "Anthropic",
    baseUrl: "https://api.anthropic.com/v1",
    models: ["claude-opus-5", "claude-sonnet-5", "claude-fable-5"],
  },
  deepseek: {
    label: "DeepSeek",
    baseUrl: "https://api.deepseek.com/v1",
    models: ["deepseek-v4-pro", "deepseek-v4-flash"],
  },
  ollama: {
    label: "Ollama (本地)",
    baseUrl: "http://localhost:11434",
    models: ["qwen3:8b", "llama4:8b", "mistral-nemo:12b"],
  },
  custom: {
    label: "OpenAI 兼容接口",
    baseUrl: "",
    models: [],
  },
};

export function SettingsModal({ open, onClose, initialConfig, onSave }: Props) {
  const [provider, setProvider] = useState<ModelConfig["provider"]>(initialConfig?.provider ?? "openai");
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState(initialConfig?.baseUrl ?? providerPresets.openai.baseUrl);
  const [model, setModel] = useState(initialConfig?.model ?? providerPresets.openai.models[0]);

  useEffect(() => {
    if (!open) return;
    const next = initialConfig?.provider ?? "openai";
    setProvider(next);
    setApiKey("");
    setBaseUrl(initialConfig?.baseUrl ?? providerPresets[next].baseUrl);
    setModel(initialConfig?.model ?? providerPresets[next].models[0] ?? "");
  }, [open, initialConfig]);

  if (!open) return null;

  const preset = providerPresets[provider];

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>模型设置</h2>
          <button className="icon-button" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <div className="modal-body">
          <p className="panel-muted">
            正文只存在本机。API Key 进密钥链，不进作品库。没配模型也能继续写。以后若有付费同步，到期不会锁已有稿件。
          </p>
          <label className="field">
            <span className="field-label">
              <Server size={14} />
              模型提供方
            </span>
            <div className="provider-grid">
              {Object.entries(providerPresets).map(([key, preset]) => (
                <button
                  key={key}
                  className={`provider-option ${provider === key ? "active" : ""}`}
                  onClick={() => {
                    setProvider(key as ModelConfig["provider"]);
                    setBaseUrl(preset.baseUrl);
                    setModel(preset.models[0] || "");
                  }}
                >
                  {preset.label}
                </button>
              ))}
            </div>
          </label>

          <label className="field">
            <span className="field-label">
              <Key size={14} />
              API Key
            </span>
            <input
              type="password"
              className="text-input"
              placeholder={
                initialConfig?.apiKeySet
                  ? "已保存在系统密钥链，留空则保持不变"
                  : provider === "ollama"
                    ? "本地模型无需 Key"
                    : "sk-..."
              }
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
            />
          </label>

          <label className="field">
            <span className="field-label">
              <Globe size={14} />
              Base URL
            </span>
            <input
              type="text"
              className="text-input"
              placeholder={preset.baseUrl || "https://your-api.example.com/v1"}
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
            />
          </label>

          <label className="field">
            <span className="field-label">
              <Cpu size={14} />
              模型
            </span>
            {preset.models.length > 0 ? (
              <select
                className="text-input"
                value={model}
                onChange={(e) => setModel(e.target.value)}
              >
                {preset.models.map((m) => (
                  <option key={m} value={m}>
                    {m}
                  </option>
                ))}
              </select>
            ) : (
              <input
                type="text"
                className="text-input"
                placeholder="model-name"
                value={model}
                onChange={(e) => setModel(e.target.value)}
              />
            )}
          </label>
        </div>

        <div className="modal-footer">
          <button className="action-button ghost" onClick={onClose}>
            取消
          </button>
          <button
            className="action-button primary"
            onClick={() => {
              onSave({
                provider,
                apiKey,
                baseUrl,
                model,
                apiKeySet: Boolean(apiKey) || Boolean(initialConfig?.apiKeySet),
              });
              onClose();
            }}
          >
            保存
          </button>
        </div>
      </div>
    </div>
  );
}
