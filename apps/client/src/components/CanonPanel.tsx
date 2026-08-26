import type { ReactNode } from "react";
import { Check, ScrollText, X } from "lucide-react";
import { CanonProposal, EntityKind } from "../types";

const KIND_LABELS: Record<EntityKind, string> = {
  character: "人物",
  location: "地点",
  organization: "组织",
  item: "物",
  ability: "能力",
  worldRule: "世界规则",
};

const PREDICATE_LABELS: Record<string, string> = {
  appearsAsSpeaker: "作为说话人出现",
  titledWork: "典籍 / 物名",
  mentionedLocation: "提及地点",
};

interface Props {
  chapterReady: boolean;
  busy: boolean;
  error: string | null;
  candidates: CanonProposal[];
  accepted: CanonProposal[];
  onExtract: () => void;
  onReview: (factId: string, accept: boolean) => void;
}

export function CanonPanel({
  chapterReady,
  busy,
  error,
  candidates,
  accepted,
  onExtract,
  onReview,
}: Props) {
  return (
    <div className="panel-content">
      <div className="panel-heading">
        <h3>正史</h3>
        <button
          className="mini-button"
          disabled={!chapterReady || busy}
          onClick={onExtract}
          title="从当前章节抽取候选"
        >
          <ScrollText size={12} />
          {busy ? "处理中" : "从本章提取"}
        </button>
      </div>
      <p className="canon-lead">
        抽取只生成候选。你确认后才会进入正史，浮带和连续性检查只读已接受的事实。
      </p>
      {error && <div className="tree-empty">{error}</div>}

      <h3 className="jobs-heading">待确认</h3>
      {candidates.length === 0 && <div className="empty-state">暂无候选。打开章节后点「从本章提取」。</div>}
      {candidates.map((item) => (
        <CanonCard
          key={item.factId}
          item={item}
          actions={
            <>
              <button className="mini-button" title="接受" onClick={() => onReview(item.factId, true)}>
                <Check size={12} />
                接受
              </button>
              <button className="mini-button" title="拒绝" onClick={() => onReview(item.factId, false)}>
                <X size={12} />
                拒绝
              </button>
            </>
          }
        />
      ))}

      <h3 className="jobs-heading">已入正史</h3>
      {accepted.length === 0 && <div className="empty-state">还没有已确认的设定。</div>}
      {accepted.map((item) => (
        <CanonCard key={item.factId} item={item} />
      ))}
    </div>
  );
}

function CanonCard({ item, actions }: { item: CanonProposal; actions?: ReactNode }) {
  return (
    <div className="context-card canon-card">
      <div className="context-card-title">
        <span className="canon-kind">{KIND_LABELS[item.entityKind] ?? item.entityKind}</span>
        {item.entityName}
      </div>
      <p>
        {PREDICATE_LABELS[item.predicate] ?? item.predicate}
        {item.object && item.object !== item.entityName ? ` · ${item.object}` : ""}
      </p>
      {item.quote && <p className="canon-quote">「{item.quote}」</p>}
      {actions && <div className="canon-actions">{actions}</div>}
    </div>
  );
}
