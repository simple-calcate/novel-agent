import { t, type MessageKey } from "../i18n";

/** Stable match-reason codes on the wire. UI formats them; engines never emit prose. */
export const MATCH_REASON_CODES = [
  "title",
  "alias",
  "core",
  "keyword",
  "lookback",
  "lookbackAlias",
  "retrieve",
] as const;

export type MatchReasonCode = (typeof MATCH_REASON_CODES)[number];

const REASON_KEYS: Record<MatchReasonCode, MessageKey> = {
  title: "match.reason.title",
  alias: "match.reason.alias",
  core: "match.reason.core",
  keyword: "match.reason.keyword",
  lookback: "match.reason.lookback",
  lookbackAlias: "match.reason.lookbackAlias",
  retrieve: "match.reason.retrieve",
};

export function isMatchReasonCode(value: string): value is MatchReasonCode {
  return (MATCH_REASON_CODES as readonly string[]).includes(value);
}

/** Encode `title:林晚`. Term may contain colons; split only on the first one. */
export function encodeMatchReason(code: MatchReasonCode, term: string): string {
  return `${code}:${term}`;
}

export function parseMatchReason(raw: string): { code: MatchReasonCode; term: string } | null {
  const sep = raw.indexOf(":");
  if (sep <= 0) return null;
  const code = raw.slice(0, sep);
  const term = raw.slice(sep + 1);
  if (!term || !isMatchReasonCode(code)) return null;
  return { code, term };
}

export function formatMatchReason(raw: string): string {
  const parsed = parseMatchReason(raw);
  if (!parsed) return raw;
  return t(REASON_KEYS[parsed.code], { term: parsed.term });
}
