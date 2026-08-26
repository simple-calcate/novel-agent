import { PreferenceRule } from "../types";

interface Props {
  rules: PreferenceRule[];
  onToggle: (rule: PreferenceRule, disabled: boolean) => void;
}

export function PreferencePanel({ rules, onToggle }: Props) {
  if (rules.length === 0) {
    return <p className="panel-muted">拒绝一次续写后，偏好会出现在这里。下次续写会写进提示。</p>;
  }
  return (
    <ul className="preference-list">
      {rules.map((rule) => (
        <li key={rule.id} className={rule.status === "disabled" ? "disabled" : ""}>
          <div>
            <strong>{rule.status === "confirmed" ? "已确认" : rule.status === "disabled" ? "已停用" : "候选"}</strong>
            <p>{rule.rule}</p>
          </div>
          <button
            className="text-button"
            onClick={() => onToggle(rule, rule.status !== "disabled")}
          >
            {rule.status === "disabled" ? "启用" : "停用"}
          </button>
        </li>
      ))}
    </ul>
  );
}
