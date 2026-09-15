import { useEffect, useState } from "react";
import { X, Key, Cpu, Globe, Server, Languages } from "lucide-react";

import { ModelConfig } from "../types";
import { useI18n, type Locale } from "../i18n";

export type { ModelConfig };

interface Props {
  open: boolean;
  onClose: () => void;
  initialConfig: ModelConfig | null;
  onSave: (config: ModelConfig) => void;
}

const providerPresets: Record<string, { baseUrl: string; models: string[] }> = {
  openai: {
    baseUrl: "https://api.openai.com/v1",
    models: ["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna", "gpt-5.5"],
  },
  anthropic: {
    baseUrl: "https://api.anthropic.com/v1",
    models: ["claude-opus-5", "claude-sonnet-5", "claude-fable-5"],
  },
  deepseek: {
    baseUrl: "https://api.deepseek.com/v1",
    models: ["deepseek-v4-pro", "deepseek-v4-flash"],
  },
  ollama: {
    baseUrl: "http://localhost:11434",
    models: ["qwen3:8b", "llama4:8b", "mistral-nemo:12b"],
  },
  custom: {
    baseUrl: "",
    models: [],
  },
};

export function SettingsModal({ open, onClose, initialConfig, onSave }: Props) {
  const { t, locale, setLocale, options } = useI18n();
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
  const providerLabel = (key: string) => {
    if (key === "ollama") return t("settings.ollama");
    if (key === "custom") return t("settings.custom");
    if (key === "openai") return "OpenAI";
    if (key === "anthropic") return "Anthropic";
    if (key === "deepseek") return "DeepSeek";
    return key;
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>{t("settings.title")}</h2>
          <button className="icon-button" onClick={onClose} title={t("common.close")}>
            <X size={16} />
          </button>
        </div>

        <div className="modal-body">
          <label className="field">
            <span className="field-label">
              <Languages size={14} />
              {t("locale.label")}
            </span>
            <div className="provider-grid">
              {options.map((option) => (
                <button
                  key={option.id}
                  type="button"
                  className={`provider-option ${locale === option.id ? "active" : ""}`}
                  onClick={() => setLocale(option.id as Locale)}
                >
                  {option.nativeLabel}
                </button>
              ))}
            </div>
            <p className="panel-muted">{t("locale.hint")}</p>
          </label>

          <p className="panel-muted">{t("settings.lead")}</p>
          <label className="field">
            <span className="field-label">
              <Server size={14} />
              {t("settings.provider")}
            </span>
            <div className="provider-grid">
              {Object.entries(providerPresets).map(([key, item]) => (
                <button
                  key={key}
                  className={`provider-option ${provider === key ? "active" : ""}`}
                  onClick={() => {
                    setProvider(key as ModelConfig["provider"]);
                    setBaseUrl(item.baseUrl);
                    setModel(item.models[0] || "");
                  }}
                >
                  {providerLabel(key)}
                </button>
              ))}
            </div>
          </label>

          <label className="field">
            <span className="field-label">
              <Key size={14} />
              {t("settings.apiKey")}
            </span>
            <input
              type="password"
              className="text-input"
              placeholder={
                initialConfig?.apiKeySet
                  ? t("settings.apiKeySaved")
                  : provider === "ollama"
                    ? t("settings.apiKeyLocal")
                    : "sk-..."
              }
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
            />
          </label>

          <label className="field">
            <span className="field-label">
              <Globe size={14} />
              {t("settings.baseUrl")}
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
              {t("settings.model")}
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
            {t("common.cancel")}
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
            {t("common.save")}
          </button>
        </div>
      </div>
    </div>
  );
}
