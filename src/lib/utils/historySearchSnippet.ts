import { truncateAroundMatch } from "$lib/utils/highlightMatches";
import type { FieldMatch, SearchField, SearchResult } from "$lib/types/historySearch";

const SNIPPET_RADIUS = 80;

export function displayName(r: SearchResult): string {
  const e = r.entry;
  return e.title ?? e.skill_name ?? "Chat";
}

function snippetSourceFor(matches: readonly FieldMatch[]): SearchField | null {
  const input = matches.find((m) => m.field === "input_content" && m.indices.length > 0);
  if (input) return "input_content";
  const output = matches.find((m) => m.field === "output_content" && m.indices.length > 0);
  if (output) return "output_content";
  return null;
}

export function snippetFor(
  r: SearchResult,
): { field: SearchField; text: string; matches: FieldMatch[] } | null {
  const source = snippetSourceFor(r.matches);
  if (!source) return null;
  const e = r.entry;
  const raw = source === "input_content" ? (e.input_content ?? "") : (e.output_content ?? "");
  if (!raw) return null;
  const truncated = truncateAroundMatch(raw, r.matches, source, SNIPPET_RADIUS);
  if (!truncated.text) return null;
  return { field: source, text: truncated.text, matches: truncated.matches };
}

export function formatTimestamp(r: SearchResult): string {
  const e = r.entry;
  const raw = e.updated_at ?? e.created_at ?? e.timestamp;
  const date = new Date(raw);
  if (isNaN(date.getTime())) return raw;
  const now = new Date();
  const startOfToday = new Date(now);
  startOfToday.setHours(0, 0, 0, 0);
  if (date.getTime() >= startOfToday.getTime()) return "Today";
  const diffDays = Math.floor((startOfToday.getTime() - date.getTime()) / 86400000);
  if (diffDays < 1) return "Yesterday";
  if (diffDays < 7) return `${diffDays + 1}d ago`;
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}
