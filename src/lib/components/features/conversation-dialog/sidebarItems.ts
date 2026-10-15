import type { HistoryEntry } from "$lib/types";

export type SidebarItem = { kind: "history"; entry: HistoryEntry };

export type SidebarIcon = "speech" | "multi-turn" | "single";

function entrySortKey(entry: HistoryEntry): string {
  return entry.updated_at ?? entry.created_at ?? entry.timestamp ?? "";
}

export function buildSidebarItems(conversations: HistoryEntry[]): SidebarItem[] {
  return conversations
    .map((entry): SidebarItem => ({ kind: "history", entry }))
    .sort((a, b) => entrySortKey(b.entry).localeCompare(entrySortKey(a.entry)));
}

export function itemId(item: SidebarItem): string {
  return item.entry.id;
}

export function itemTitle(item: SidebarItem): string {
  return item.entry.title ?? item.entry.skill_name ?? "New chat";
}

export function itemIcon(item: SidebarItem): SidebarIcon {
  const entry = item.entry;
  if (entry.entry_type === "speech") return "speech";
  if (entry.is_multi_turn) return "multi-turn";
  return "single";
}

export function itemTimestamp(item: SidebarItem): string | null {
  const entry = item.entry;
  const raw = entry.updated_at ?? entry.created_at ?? entry.timestamp;
  return raw ? formatTimestamp(raw) : null;
}

export function formatTimestamp(raw: string): string {
  const date = new Date(raw);
  if (isNaN(date.getTime())) return raw;
  const diffMs = Date.now() - date.getTime();
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1) return "Just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHours = Math.floor(diffMin / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}
