import { ContextHint } from "../types";
import { AlertTriangle, BookMarked, Eye, Flame, Pin, X } from "lucide-react";

interface Props {
  hints: ContextHint[];
}

const kindLabels: Record<ContextHint["kind"], string> = {
  characterState: "人物状态",
  worldRule: "世界规则",
  timelineConstraint: "时间线",
  openForeshadowing: "未兑现伏笔",
  plotHook: "剧情钩子",
  preference: "写作偏好",
  continuityRisk: "连续性风险",
};

const kindIcons: Record<ContextHint["kind"], typeof BookMarked> = {
  characterState: BookMarked,
  worldRule: BookMarked,
  timelineConstraint: BookMarked,
  openForeshadowing: Flame,
  plotHook: Eye,
  preference: BookMarked,
  continuityRisk: AlertTriangle,
};

export function ContextRail({ hints }: Props) {
  if (hints.length === 0) return null;

  return (
    <div className="context-rail">
      {hints.map((hint) => {
        const Icon = kindIcons[hint.kind];
        return (
          <article key={hint.id} className={`hint-card kind-${hint.kind}`}>
            <div className="hint-header">
              <Icon size={13} />
              <span className="hint-kind">{kindLabels[hint.kind]}</span>
              <span className="hint-score">{Math.round(hint.confidence * 100)}%</span>
            </div>
            <h4>{hint.title}</h4>
            <p>{hint.summary}</p>
            <div className="hint-footer">
              <span>{hint.matchReason}</span>
              <div className="hint-actions">
                <button title="钉住">
                  <Pin size={12} />
                </button>
                <button title="忽略">
                  <X size={12} />
                </button>
              </div>
            </div>
          </article>
        );
      })}
    </div>
  );
}
