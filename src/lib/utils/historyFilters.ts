import type { HistoryStatusFilter, HistoryTypeFilter } from "$lib/types/historySearch";

export function formatActiveFilters(s: {
  query: string;
  typeFilter: HistoryTypeFilter;
  statusFilter: HistoryStatusFilter;
}): string {
  const parts: string[] = [];
  if (s.query.trim()) parts.push(`Query: "${s.query.trim()}"`);
  if (s.typeFilter !== "all") parts.push(`Type: ${typeLabel(s.typeFilter)}`);
  if (s.statusFilter !== "all") parts.push(`Status: ${capitalize(s.statusFilter)}`);
  return parts.join(" · ");
}

function typeLabel(t: HistoryTypeFilter): string {
  return t === "quick_action" ? "Quick Action" : capitalize(t);
}

function capitalize(s: string): string {
  return s ? s[0].toUpperCase() + s.slice(1) : s;
}
