import { PreferenceRule } from "../types";
import { useI18n } from "../i18n";

interface Props {
  rules: PreferenceRule[];
  onToggle: (rule: PreferenceRule, disabled: boolean) => void;
}

export function PreferencePanel({ rules, onToggle }: Props) {
  const { t } = useI18n();
  if (rules.length === 0) {
    return <p className="panel-muted">{t("preference.empty")}</p>;
  }
  return (
    <ul className="preference-list">
      {rules.map((rule) => (
        <li key={rule.id} className={rule.status === "disabled" ? "disabled" : ""}>
          <div>
            <strong>
              {rule.status === "confirmed"
                ? t("preference.confirmed")
                : rule.status === "disabled"
                  ? t("preference.disabled")
                  : t("preference.candidate")}
            </strong>
            <p>{rule.rule}</p>
          </div>
          <button
            className="text-button"
            onClick={() => onToggle(rule, rule.status !== "disabled")}
          >
            {rule.status === "disabled" ? t("preference.enable") : t("preference.disable")}
          </button>
        </li>
      ))}
    </ul>
  );
}
