import { ContextHint, StoryEntry } from "../types";

const STOPWORDS = new Set([
  "然后",
  "于是",
  "只是",
  "可是",
  "但是",
  "这时",
  "此时",
  "忽然",
  "突然",
  "终于",
  "其实",
  "因为",
  "所以",
  "如果",
  "虽然",
  "他们",
  "她们",
  "我们",
  "自己",
  "这个",
  "那个",
  "什么",
  "没有",
  "不是",
  "已经",
  "还是",
  "或者",
  "以及",
  "里面",
  "还有",
  "一个",
  "现在",
  "后来",
  "开始",
  "继续",
  "出现",
  "时候",
  "地方",
  "东西",
  "补充",
  "说明",
  "预先",
  "设定",
  "人物",
  "伏笔",
  "条目",
  "终年",
  "夜晚",
  "海面",
  "进来",
  "出去",
  "看见",
  "听到",
  "知道",
  "觉得",
  "走过",
  "走进",
  "来到",
  "抵达",
]);

const SPLITTERS = /[，。；、,.!?！？：:\n\t 「」“”'（）《》—…·的地得]/;

export function splitTitleAndAliases(raw: string): { title: string; aliases: string[] } {
  const parts = raw
    .split(/[、,，/／;；]/)
    .map((part) => part.trim())
    .filter(Boolean);
  return { title: parts[0] ?? "", aliases: parts.slice(1) };
}

export function matchStoryEntries(
  current: string,
  lookback: string,
  entries: StoryEntry[],
  revision: number,
): ContextHint[] {
  const hints: ContextHint[] = [];
  for (const entry of entries) {
    const hit = matchEntry(current, lookback, entry);
    if (!hit) continue;
    const kind =
      entry.kind === "character"
        ? "characterState"
        : entry.kind === "foreshadow"
          ? "openForeshadowing"
          : "worldRule";
    hints.push({
      id: entry.id,
      kind,
      title: entry.title,
      summary: entry.summary || `${entry.title} · 预先设定`,
      sourceLabel: entry.kind,
      matchReason: hit.reason,
      confidence: hit.score,
      score: hit.score,
      generation: revision,
      revision,
    });
  }
  const kindRank: Record<ContextHint["kind"], number> = {
    characterState: 0,
    worldRule: 1,
    timelineConstraint: 1,
    openForeshadowing: 2,
    plotHook: 2,
    preference: 1,
    continuityRisk: 1,
  };
  return hints
    .sort((left, right) => right.score - left.score || kindRank[left.kind] - kindRank[right.kind])
    .slice(0, 6);
}

function matchEntry(
  current: string,
  lookback: string,
  entry: StoryEntry,
): { score: number; reason: string } | null {
  const title = entry.title.trim();
  if ([...title].length < 2) return null;
  const aliases = [
    ...(entry.aliases ?? []),
    ...splitTitleAndAliases(title).aliases,
  ]
    .map((alias) => alias.trim())
    .filter((alias) => [...alias].length >= 2 && alias !== title);
  const currentL = normalize(current);
  const lookbackL = normalize(lookback);
  const hits: { score: number; reason: string }[] = [];

  const titleAt = findTerm(currentL, title);
  if (titleAt != null) {
    hits.push({ score: 0.98 * positionBoost(currentL, titleAt), reason: `出现名称「${title}」` });
  }
  for (const alias of aliases) {
    const at = findTerm(currentL, alias);
    if (at != null) {
      hits.push({ score: 0.9 * positionBoost(currentL, at), reason: `出现别名「${alias}」` });
    }
  }
  for (const core of titleCores(title)) {
    const at = findTerm(currentL, core);
    if (at != null) {
      hits.push({ score: 0.72 * positionBoost(currentL, at), reason: `提到「${core}」` });
    }
  }
  const keywordHits: { score: number; reason: string }[] = [];
  for (const keyword of summaryKeywords(entry.summary, title, aliases)) {
    const at = findTerm(currentL, keyword);
    if (at != null) {
      keywordHits.push({
        score: 0.58 * positionBoost(currentL, at),
        reason: `设定里提到「${keyword}」`,
      });
    }
  }
  if (keywordHits.length > 0) {
    keywordHits.sort((a, b) => b.score - a.score);
    const hit = keywordHits[0];
    hits.push({
      score: Math.min(0.76, hit.score + 0.06 * (keywordHits.length - 1)),
      reason: hit.reason,
    });
  }

  if (hits.length === 0) {
    if (findTerm(lookbackL, title) != null) {
      return { score: 0.6, reason: `上一段出现「${title}」` };
    }
    for (const alias of aliases) {
      if (findTerm(lookbackL, alias) != null) {
        return { score: 0.56, reason: `上一段出现别名「${alias}」` };
      }
    }
    return null;
  }
  hits.sort((left, right) => right.score - left.score);
  const best = hits[0];
  if (hits.length > 1) best.score = Math.min(1, best.score + 0.04 * (hits.length - 1));
  return best;
}

function titleCores(title: string): string[] {
  const chars = [...title];
  const n = chars.length;
  if (n < 3) return [];
  const cores: string[] = [];
  const suffix2 = chars.slice(n - 2).join("");
  if (n >= 3 && !STOPWORDS.has(suffix2) && suffix2 !== title) cores.push(suffix2);
  if (n >= 4) {
    const suffix3 = chars.slice(n - 3).join("");
    if (!STOPWORDS.has(suffix3)) cores.push(suffix3);
  }
  return cores;
}

function summaryKeywords(summary: string, title: string, aliases: string[]): string[] {
  const tokens: string[] = [];
  for (const chunk of summary.split(SPLITTERS)) {
    pushToken(tokens, chunk, title, aliases);
  }
  tokens.sort((a, b) => [...b].length - [...a].length);
  return [...new Set(tokens)];
}

function pushToken(tokens: string[], chunk: string, title: string, aliases: string[]) {
  const text = chunk.trim();
  const chars = [...text];
  const n = chars.length;
  if (n < 2 || n > 12) return;
  acceptToken(tokens, text, title, aliases);
  if (n >= 4) {
    acceptToken(tokens, chars.slice(0, 2).join(""), title, aliases);
    acceptToken(tokens, chars.slice(n - 2).join(""), title, aliases);
    acceptToken(tokens, chars.slice(n - 3).join(""), title, aliases);
  } else if (n === 3) {
    acceptToken(tokens, chars.slice(0, 2).join(""), title, aliases);
    acceptToken(tokens, chars.slice(n - 2).join(""), title, aliases);
  }
}

function acceptToken(tokens: string[], token: string, title: string, aliases: string[]) {
  if ([...token].length < 2 || STOPWORDS.has(token)) return;
  if (token === title || aliases.includes(token) || title.includes(token)) return;
  if (!tokens.includes(token)) tokens.push(token);
}

function normalize(text: string): string {
  return text.toLowerCase().replace(/[\u3000\u00a0]/g, " ");
}

function findTerm(haystack: string, term: string): number | null {
  const needle = normalize(term);
  if ([...needle].length < 2) return null;
  const index = haystack.indexOf(needle);
  return index < 0 ? null : index;
}

function positionBoost(text: string, index: number): number {
  return Math.min(1, Math.max(0.8, 1 - (index / Math.max(text.length, 1)) * 0.2));
}
