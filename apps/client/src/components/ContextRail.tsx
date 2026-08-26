import { ContextHint } from "../types";
import { BookMarked, Flame, User } from "lucide-react";

interface Props {
  hints: ContextHint[];
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
            </div>
            <h4>{hint.title}</h4>
            <p>{hint.summary}</p>
          </article>
        );
      })}
    </div>
  );
}
