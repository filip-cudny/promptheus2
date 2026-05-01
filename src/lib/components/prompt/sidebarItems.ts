import type { HistoryEntry } from "$lib/types";
import type { TabState } from "$lib/types/conversation";

export type SidebarItem =
  | { kind: "draft"; tab: TabState }
  | { kind: "open"; tab: TabState; entry: HistoryEntry }
  | { kind: "history"; entry: HistoryEntry };

export type SidebarIcon = "speech" | "multi-turn" | "single" | "draft";

export function buildSidebarItems(
  tabs: TabState[],
  conversations: HistoryEntry[],
): SidebarItem[] {
  const openEntryIds = new Set<string>();
  const result: SidebarItem[] = [];

  for (const tab of tabs) {
    if (tab.history_entry_id) {
      openEntryIds.add(tab.history_entry_id);
      const entry = conversations.find((e) => e.id === tab.history_entry_id);
      if (entry) result.push({ kind: "open", tab, entry });
      else result.push({ kind: "draft", tab });
    } else {
      result.push({ kind: "draft", tab });
    }
  }

  for (const entry of conversations) {
    if (!openEntryIds.has(entry.id)) result.push({ kind: "history", entry });
  }

  result.sort((a, b) => {
    const tsA = itemSortKey(a);
    const tsB = itemSortKey(b);
    if (tsA === null && tsB === null) return 0;
    if (tsA === null) return -1;
    if (tsB === null) return 1;
    return tsB.localeCompare(tsA);
  });

  return result;
}

function itemSortKey(item: SidebarItem): string | null {
  if (item.kind === "draft") return null;
  return item.entry.updated_at ?? item.entry.created_at ?? item.entry.timestamp ?? null;
}

export function itemId(item: SidebarItem): string {
  return item.kind === "history" ? item.entry.id : item.tab.tab_id;
}

export function itemTitle(item: SidebarItem): string {
  if (item.kind === "draft") return item.tab.tab_name ?? "New chat";
  return item.entry.title ?? item.entry.skill_name ?? "New chat";
}

export function itemIcon(
  item: SidebarItem,
  isTabClean: (tab: TabState) => boolean,
): SidebarIcon {
  if (item.kind === "draft" && !isTabClean(item.tab)) return "draft";
  if (item.kind === "draft") return "single";
  const entry = item.entry;
  if (entry.entry_type === "speech") return "speech";
  if (entry.is_multi_turn) return "multi-turn";
  return "single";
}

export function itemTimestamp(item: SidebarItem): string | null {
  if (item.kind === "draft") return null;
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
