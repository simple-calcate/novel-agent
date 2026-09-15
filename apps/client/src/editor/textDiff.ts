import { formatDiffSummary } from "../i18n";
import { DiffChangeTag, DiffLine, DiffSpan, RevisionDiff } from "../types";

export interface TextDiffResult {
  summary: string;
  insertedChars: number;
  deletedChars: number;
  ratio: number;
  lines: DiffLine[];
}

/** 浏览器预览用的行级 LCS；桌面走 Rust `similar`。 */
export function diffTexts(oldText: string, newText: string): TextDiffResult {
  const oldLines = splitLines(oldText);
  const newLines = splitLines(newText);
  const ops = lineOps(oldLines, newLines);
  const lines: DiffLine[] = [];
  let insertedChars = 0;
  let deletedChars = 0;
  let equalChars = 0;
  let oldIndex = 0;
  let newIndex = 0;

  for (const op of ops) {
    if (op.tag === "equal") {
      const text = oldLines[op.old] ?? "";
      equalChars += [...text].length;
      lines.push(makeLine("equal", text, oldIndex, newIndex));
      oldIndex += 1;
      newIndex += 1;
    } else if (op.tag === "delete") {
      const text = oldLines[op.old] ?? "";
      deletedChars += [...text].length;
      lines.push(makeLine("delete", text, oldIndex, null));
      oldIndex += 1;
    } else {
      const text = newLines[op.new] ?? "";
      insertedChars += [...text].length;
      lines.push(makeLine("insert", text, null, newIndex));
      newIndex += 1;
    }
  }

  const identical = oldText === newText;
  const total = insertedChars + deletedChars + equalChars;
  return {
    summary: formatSummary(insertedChars, deletedChars, identical),
    insertedChars,
    deletedChars,
    ratio: total === 0 ? 1 : equalChars / total,
    lines,
  };
}

export function revisionDiff(
  chapterId: string,
  fromRevision: number,
  toRevision: number,
  oldText: string,
  newText: string,
): RevisionDiff {
  return {
    chapterId,
    fromRevision,
    toRevision,
    ...diffTexts(oldText, newText),
  };
}

export function previewText(text: string): string {
  const collapsed = text.split(/\s+/).filter(Boolean).join(" ");
  const chars = [...collapsed];
  if (chars.length <= 48) return collapsed;
  return `${chars.slice(0, 48).join("")}…`;
}

function makeLine(
  tag: DiffChangeTag,
  text: string,
  oldIndex: number | null,
  newIndex: number | null,
): DiffLine {
  const spans: DiffSpan[] = text ? [{ tag, text }] : [];
  return { tag, oldIndex, newIndex, text, spans };
}

function splitLines(text: string): string[] {
  if (text.length === 0) return [];
  return text.split("\n");
}

function formatSummary(inserted: number, deleted: number, identical: boolean): string {
  return formatDiffSummary(inserted, deleted, identical);
}

interface LineOp {
  tag: DiffChangeTag;
  old: number;
  new: number;
}

function lineOps(oldLines: string[], newLines: string[]): LineOp[] {
  const n = oldLines.length;
  const m = newLines.length;
  const dp: number[][] = Array.from({ length: n + 1 }, () => Array(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i -= 1) {
    for (let j = m - 1; j >= 0; j -= 1) {
      dp[i][j] =
        oldLines[i] === newLines[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
    }
  }
  const ops: LineOp[] = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (oldLines[i] === newLines[j]) {
      ops.push({ tag: "equal", old: i, new: j });
      i += 1;
      j += 1;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      ops.push({ tag: "delete", old: i, new: j });
      i += 1;
    } else {
      ops.push({ tag: "insert", old: i, new: j });
      j += 1;
    }
  }
  while (i < n) {
    ops.push({ tag: "delete", old: i, new: j });
    i += 1;
  }
  while (j < m) {
    ops.push({ tag: "insert", old: i, new: j });
    j += 1;
  }
  return ops;
}
