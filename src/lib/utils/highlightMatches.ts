import type { FieldMatch, SearchField } from "$lib/types/historySearch";

export interface TruncateResult {
  text: string;
  matches: FieldMatch[];
}

const WORD_BOUNDARY_MAX_SHIFT = 20;

function isWordChar(c: string): boolean {
  return /\S/.test(c);
}

function snapToWordBoundary(
  chars: readonly string[],
  idx: number,
  direction: "right" | "left",
  maxShift: number,
): number {
  if (idx <= 0 || idx >= chars.length) return idx;
  if (!isWordChar(chars[idx - 1]) || !isWordChar(chars[idx])) return idx;
  if (direction === "right") {
    const limit = Math.min(chars.length, idx + maxShift);
    let i = idx;
    while (i < limit && isWordChar(chars[i])) i++;
    while (i < limit && !isWordChar(chars[i])) i++;
    return i;
  } else {
    const limit = Math.max(0, idx - maxShift);
    let i = idx;
    while (i > limit && isWordChar(chars[i - 1])) i--;
    while (i > limit && !isWordChar(chars[i - 1])) i--;
    return i;
  }
}

export function truncateAroundMatch(
  text: string,
  matches: readonly FieldMatch[],
  field: SearchField,
  max: number,
): TruncateResult {
  if (!text) return { text: "", matches: [...matches] };
  const chars = Array.from(text);
  if (chars.length <= max) return { text, matches: [...matches] };

  const target = matches.find((m) => m.field === field);
  const firstIdx = target?.indices.length ? Math.min(...target.indices) : -1;
  const threshold = Math.floor(max / 2);

  let start: number;
  let end: number;
  if (firstIdx < 0 || firstIdx <= threshold) {
    start = 0;
    end = Math.min(chars.length, max);
  } else {
    const headSlack = Math.floor(max / 4);
    start = Math.max(0, firstIdx - headSlack);
    end = Math.min(chars.length, start + max);
  }

  if (start > 0) {
    const snapped = snapToWordBoundary(chars, start, "right", WORD_BOUNDARY_MAX_SHIFT);
    if (firstIdx < 0 || snapped <= firstIdx) start = snapped;
  }
  if (end < chars.length) {
    const snapped = snapToWordBoundary(chars, end, "left", WORD_BOUNDARY_MAX_SHIFT);
    if (snapped > firstIdx) end = snapped;
  }

  const sliced = chars.slice(start, end);
  const prefix = start > 0 ? "…" : "";
  const suffix = end < chars.length ? "…" : "";
  const newText = prefix + sliced.join("") + suffix;
  const offset = start - prefix.length;

  const translated: FieldMatch[] = matches.map((m) => {
    if (m.field !== field) return m;
    const idx: number[] = [];
    for (const i of m.indices) {
      const moved = i - offset;
      if (moved >= prefix.length && moved < prefix.length + sliced.length) {
        idx.push(moved);
      }
    }
    return { field: m.field, indices: idx };
  });

  return { text: newText, matches: translated };
}

export function highlightFor(
  text: string,
  matches: readonly FieldMatch[],
  fields: readonly SearchField[],
): string {
  if (!text) return "";
  if (!matches?.length) return escapeHtml(text);
  const m = fields.map((f) => matches.find((x) => x.field === f)).find(Boolean);
  if (!m) return escapeHtml(text);
  return interleave(text, m.indices);
}

function escapeHtml(s: string): string {
  return s.replace(
    /[&<>"']/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&#39;" })[c]!,
  );
}

function interleave(text: string, indices: readonly number[]): string {
  if (!indices.length) return escapeHtml(text);
  const sorted = [...indices].sort((a, b) => a - b);
  const chars = Array.from(text);
  const result: string[] = [];
  let cursor = 0;
  let i = 0;
  while (i < sorted.length) {
    const start = sorted[i];
    if (start < cursor || start >= chars.length) {
      i++;
      continue;
    }
    let end = start + 1;
    while (i + 1 < sorted.length && sorted[i + 1] === end && end < chars.length) {
      end++;
      i++;
    }
    if (start > cursor) result.push(escapeHtml(chars.slice(cursor, start).join("")));
    result.push(`<mark>${escapeHtml(chars.slice(start, end).join(""))}</mark>`);
    cursor = end;
    i++;
  }
  if (cursor < chars.length) result.push(escapeHtml(chars.slice(cursor).join("")));
  return result.join("");
}
