/** 浏览器预览用的启发式抽取，与 `novel-story-model::extract` 对齐。 */

const SPEAKER_SUFFIXES = [
  "冷冷道",
  "淡淡道",
  "低声道",
  "缓缓道",
  "说道",
  "问道",
  "笑道",
  "喝道",
  "叹道",
  "答道",
  "怒道",
  "叫道",
];

const LOCATION_PREFIXES = ["来到了", "走进了", "抵达了", "来到", "走进", "抵达"];

const NAME_STOPWORDS = new Set([
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
]);

export interface Mention {
  entityName: string;
  entityKind: "character" | "location" | "item";
  predicate: string;
  object: string;
  quote: string;
  confidence: number;
}

function isCjk(ch: string): boolean {
  const code = ch.codePointAt(0) ?? 0;
  return code >= 0x4e00 && code <= 0x9fff;
}

function precedingCjkName(text: string, at: number): string | null {
  let end = at;
  while (end > 0 && /\s/.test(text[end - 1] ?? "")) end -= 1;
  let start = end;
  let count = 0;
  while (start > 0 && count < 8) {
    const ch = text[start - 1] ?? "";
    if (isCjk(ch)) {
      start -= 1;
      count += 1;
    } else {
      break;
    }
  }
  if (count < 2) return null;
  return text.slice(start, end);
}

function followingCjkName(text: string, from: number, min: number, max: number): string | null {
  let start = from;
  while (start < text.length && /\s/.test(text[start] ?? "")) start += 1;
  let end = start;
  let count = 0;
  while (end < text.length && count < max) {
    const ch = text[end] ?? "";
    if (isCjk(ch)) {
      end += 1;
      count += 1;
    } else {
      break;
    }
  }
  if (count < min) return null;
  return text.slice(start, end);
}

function quoteAround(text: string, start: number, end: number): string {
  return text.slice(Math.max(0, start - 12), Math.min(text.length, end + 12)).replace(/\n/g, " ").trim();
}

export function extractMentions(text: string): Mention[] {
  const mentions: Mention[] = [];

  for (const suffix of SPEAKER_SUFFIXES) {
    let search = 0;
    while (search < text.length) {
      const at = text.indexOf(suffix, search);
      if (at < 0) break;
      const name = precedingCjkName(text, at);
      if (name && !NAME_STOPWORDS.has(name)) {
        const end = at + suffix.length;
        mentions.push({
          entityName: name,
          entityKind: "character",
          predicate: "appearsAsSpeaker",
          object: name,
          quote: quoteAround(text, at - name.length, end),
          confidence: 0.82,
        });
      }
      search = at + suffix.length;
    }
  }

  let search = 0;
  while (search < text.length) {
    const start = text.indexOf("《", search);
    if (start < 0) break;
    const close = text.indexOf("》", start + 1);
    if (close < 0) break;
    const title = text.slice(start + 1, close);
    if (title.length >= 2 && title.length <= 20 && !title.includes("\n")) {
      mentions.push({
        entityName: title,
        entityKind: "item",
        predicate: "titledWork",
        object: title,
        quote: quoteAround(text, start, close + 1),
        confidence: 0.78,
      });
    }
    search = close + 1;
  }

  for (const prefix of LOCATION_PREFIXES) {
    let locSearch = 0;
    while (locSearch < text.length) {
      const at = text.indexOf(prefix, locSearch);
      if (at < 0) break;
      const after = at + prefix.length;
      const name = followingCjkName(text, after, 2, 12);
      if (name && !NAME_STOPWORDS.has(name)) {
        mentions.push({
          entityName: name,
          entityKind: "location",
          predicate: "mentionedLocation",
          object: name,
          quote: quoteAround(text, at, after + name.length),
          confidence: 0.64,
        });
      }
      locSearch = after;
    }
  }

  const seen = new Set<string>();
  return mentions.filter((mention) => {
    const key = `${mention.entityKind}|${mention.entityName}|${mention.predicate}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}
