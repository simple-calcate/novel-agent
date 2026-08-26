import { Pin, X } from "lucide-react";
import { ContextHint } from "../types";
import { BookMarked, Flame, User } from "lucide-react";

interface Props {
  hints: ContextHint[];
  pinnedIds: string[];
  ignoredIds: string[];
  onPin: (id: string) => void;
  onIgnore: (id: string) => void;
}

const kindLabels: Record<ContextHint["kind"], string> = {
  characterState: "人物",
  worldRule: "设定",
  timelineConstraint: "设定",
  openForeshadowing: "伏笔",
  plotHook: "伏笔",
  preference: "设定",
  continuityRisk: "设定",
};

const kindIcons: Record<ContextHint["kind"], typeof BookMarked> = {
  characterState: User,
  worldRule: BookMarked,
  timelineConstraint: BookMarked,
  openForeshadowing: Flame,
  plotHook: Flame,
  preference: BookMarked,
  continuityRisk: BookMarked,
};

export function ContextRail({ hints, pinnedIds, ignoredIds, onPin, onIgnore }: Props) {
  const visible = hints
    .filter((hint) => !ignoredIds.includes(hint.id))
    .slice()
    .sort((left, right) => Number(pinnedIds.includes(right.id)) - Number(pinnedIds.includes(left.id)));
  if (visible.length === 0) return null;

  return (
    <div className="context-rail">
      {visible.map((hint) => {
        const Icon = kindIcons[hint.kind];
        const pinned = pinnedIds.includes(hint.id);
        return (
          <article
            key={hint.id}
            className={`hint-card kind-${hint.kind} ${pinned ? "pinned" : ""}`}
          >
            <div className="hint-header">
              <Icon size={13} />
              <span className="hint-kind">{kindLabels[hint.kind]}</span>
              <button
                className={`hint-action ${pinned ? "active" : ""}`}
                title={pinned ? "取消钉住" : "钉住"}
                onClick={() => onPin(hint.id)}
              >
                <Pin size={12} />
              </button>
              <button className="hint-action" title="忽略" onClick={() => onIgnore(hint.id)}>
                <X size={12} />
              </button>
            </div>
            <h4>{hint.title}</h4>
            <p>{hint.summary}</p>
            {hint.matchReason && <span className="hint-source">{hint.matchReason}</span>}
          </article>
        );
      })}
    </div>
  );
}
