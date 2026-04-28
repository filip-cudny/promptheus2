import type { FieldMatch, SearchField } from "$lib/types/historySearch";

export interface TruncateResult {
  text: string;
  matches: FieldMatch[];
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

  if (firstIdx < 0 || firstIdx <= threshold) {
    return { text: chars.slice(0, max).join("") + "…", matches: [...matches] };
  }

  const headSlack = Math.floor(max / 4);
  const start = Math.max(0, firstIdx - headSlack);
  const end = Math.min(chars.length, start + max);
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
  for (const idx of sorted) {
    if (idx < cursor || idx >= chars.length) continue;
    if (idx > cursor) result.push(escapeHtml(chars.slice(cursor, idx).join("")));
    result.push(`<mark>${escapeHtml(chars[idx])}</mark>`);
    cursor = idx + 1;
  }
  if (cursor < chars.length) result.push(escapeHtml(chars.slice(cursor).join("")));
  return result.join("");
}
