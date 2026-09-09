import type { StoryEntry, StoryEntryKind } from "../types";

export const STORY_TAG_KIND_LABELS = ["人物", "伏笔", "地点", "道具", "势力", "规则"] as const;
export type StoryTagKind = (typeof STORY_TAG_KIND_LABELS)[number];

const TAG_TO_ENTRY: Record<string, StoryEntryKind> = {
  人物: "character",
  伏笔: "foreshadow",
  地点: "setting",
  道具: "setting",
  势力: "setting",
  规则: "setting",
};

const SETTING_HINTS: Record<string, string[]> = {
  地点: ["港", "码头", "城", "街", "山", "村", "镇", "府", "殿", "楼", "岛", "海", "河", "巷", "院", "渡", "关", "谷"],
  道具: ["表", "剑", "刀", "信", "令", "珠", "玉", "符", "册", "盒", "铃", "镜", "怀表"],
  势力: ["司", "门", "帮", "派", "宗", "军", "盟", "会", "阁"],
  规则: ["规则", "律", "禁", "能传", "不可"],
};

export interface TagCandidate {
  id: string;
  name: string;
  summary: string;
  matchedAlias?: string;
}

export function entryKindForTag(tagKind: string): StoryEntryKind | null {
  return TAG_TO_ENTRY[tagKind] ?? null;
}

export function filterTagKindLabels(query: string, labels: readonly string[] = STORY_TAG_KIND_LABELS): string[] {
  const q = query.trim();
  if (!q) return [...labels];
  return labels.filter((label) => label.includes(q));
}

export function listTagCandidates(
  entries: StoryEntry[],
  tagKind: string,
  query: string,
  nearby = "",
): TagCandidate[] {
  const kind = entryKindForTag(tagKind);
  if (!kind) return [];
  const q = query.trim();
  const scored: Array<TagCandidate & { score: number }> = [];

  for (const entry of entries) {
    if (entry.kind !== kind) continue;
    const aliasHit = entry.aliases.find((alias) => matchesQuery(alias, q));
    const titleHit = matchesQuery(entry.title, q);
    if (q && !titleHit && !aliasHit) continue;

    let score = 0;
    if (nearby.includes(entry.title)) score += 50;
    for (const alias of entry.aliases) {
      if (nearby.includes(alias)) score += 40;
    }
    if (q) {
      if (entry.title.startsWith(q)) score += 30;
      else if (titleHit) score += 16;
      if (aliasHit?.startsWith(q)) score += 24;
      else if (aliasHit) score += 12;
    }
    const hints = SETTING_HINTS[tagKind];
    if (hints) {
      const blob = `${entry.title}${entry.summary}`;
      if (hints.some((hint) => blob.includes(hint))) score += 8;
    }
    scored.push({
      id: entry.id,
      name: entry.title,
      summary: entry.summary,
      ...( !titleHit && aliasHit ? { matchedAlias: aliasHit } : {}),
      score,
    });
  }

  scored.sort((a, b) => b.score - a.score || a.name.localeCompare(b.name, "zh"));
  return scored.map(({ score: _score, ...row }) => row);
}

function matchesQuery(text: string, query: string): boolean {
  if (!query) return true;
  return text.includes(query);
}
