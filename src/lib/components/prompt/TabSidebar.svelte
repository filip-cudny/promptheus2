<script lang="ts">
  import { X, MessageSquare, MessagesSquare, Mic, Circle } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import type { HistoryEntry } from "$lib/types";
  import type { TabState } from "$lib/types/conversation";
  import type { createConversationStore } from "$lib/stores/conversation.svelte";
  import type { getHistoryStore } from "$lib/stores/history.svelte";

  let {
    store,
    historyStore,
    open,
    onClose,
  }: {
    store: ReturnType<typeof createConversationStore>;
    historyStore: ReturnType<typeof getHistoryStore>;
    open: boolean;
    onClose: () => void;
  } = $props();

  type SidebarItem =
    | { kind: "draft"; tab: TabState }
    | { kind: "open"; tab: TabState; entry: HistoryEntry }
    | { kind: "history"; entry: HistoryEntry };

  const items = $derived.by(() => {
    const openEntryIds = new Set<string>();
    const result: SidebarItem[] = [];

    for (const tab of store.tabs) {
      if (tab.history_entry_id) {
        openEntryIds.add(tab.history_entry_id);
        const entry = historyStore.entries.find(e => e.id === tab.history_entry_id);
        if (entry) {
          result.push({ kind: "open", tab, entry });
        } else {
          result.push({ kind: "draft", tab });
        }
      } else {
        result.push({ kind: "draft", tab });
      }
    }

    for (const entry of historyStore.entries) {
      if (!openEntryIds.has(entry.id)) {
        result.push({ kind: "history", entry });
      }
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
  });

  function itemSortKey(item: SidebarItem): string | null {
    if (item.kind === "draft") return null;
    const entry = item.kind === "open" ? item.entry : item.entry;
    return entry.updated_at ?? entry.created_at ?? entry.timestamp ?? null;
  }

  function itemId(item: SidebarItem): string {
    if (item.kind === "history") return item.entry.id;
    return item.tab.tab_id;
  }

  function itemTitle(item: SidebarItem): string {
    if (item.kind === "draft") return item.tab.tab_name ?? "New chat";
    const entry = item.kind === "open" ? item.entry : item.entry;
    return entry.title ?? entry.skill_name ?? entry.input_content.slice(0, 60);
  }

  function itemTimestamp(item: SidebarItem): string | null {
    if (item.kind === "draft") return null;
    const entry = item.kind === "open" ? item.entry : item.entry;
    const raw = entry.updated_at ?? entry.created_at ?? entry.timestamp;
    return raw ? formatTimestamp(raw) : null;
  }

  function isItemActive(item: SidebarItem): boolean {
    if (item.kind === "history") {
      const tab = store.tabs.find(t => t.history_entry_id === item.entry.id);
      return tab?.tab_id === store.activeTabId;
    }
    return item.tab.tab_id === store.activeTabId;
  }

  function isDraft(item: SidebarItem): boolean {
    if (item.kind !== "draft") return false;
    return !store.isTabClean(item.tab);
  }

  function handleItemClick(item: SidebarItem) {
    if (item.kind === "history") {
      store.restoreFromHistory(item.entry.id, false);
    } else {
      store.switchTab(item.tab.tab_id);
    }
    onClose();
  }

  function handleClose(e: MouseEvent, item: SidebarItem) {
    e.stopPropagation();
    if (item.kind !== "history") {
      store.closeTab(item.tab.tab_id);
    }
  }

  function formatTimestamp(raw: string): string {
    const date = new Date(raw);
    if (isNaN(date.getTime())) return raw;
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMin = Math.floor(diffMs / 60000);
    if (diffMin < 1) return "Just now";
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHours = Math.floor(diffMin / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={onClose}></div>
{/if}

<aside class="sidebar" class:open>
  <div class="sidebar-header">
    <span class="sidebar-title">Conversations</span>
    <button class="close-btn" onclick={onClose}>
      <X size={ICON_SIZE.md} />
    </button>
  </div>

  <div class="tab-list">
    {#each items as item (itemId(item))}
      {@const ts = itemTimestamp(item)}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions a11y_no_noninteractive_element_interactions -->
      <div
        class="tab-item"
        class:active={isItemActive(item)}
        onclick={() => handleItemClick(item)}
      >
        {#if isDraft(item)}
          <Circle size={8} fill="currentColor" class="draft-dot" />
        {:else if item.kind !== "draft" && (item.kind === "history" ? item.entry : item.entry).entry_type === "speech"}
          <Mic size={ICON_SIZE.sm} />
        {:else if item.kind !== "draft" && (item.kind === "history" ? item.entry : item.entry).is_multi_turn}
          <MessagesSquare size={ICON_SIZE.sm} />
        {:else}
          <MessageSquare size={ICON_SIZE.sm} />
        {/if}
        <div class="tab-body">
          <span class="tab-name">{itemTitle(item)}</span>
          {#if ts}
            <span class="tab-meta">{ts}</span>
          {/if}
        </div>
        {#if item.kind !== "history" && store.tabs.length > 1}
          <button class="tab-close-btn" onclick={(e) => handleClose(e, item)}>
            <X size={12} />
          </button>
        {/if}
      </div>
    {/each}
  </div>
</aside>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.3);
    z-index: 199;
  }

  .sidebar {
    position: fixed;
    top: 0;
    left: 0;
    bottom: 0;
    width: 240px;
    background: rgba(5, 5, 5, 0.3);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    border-right: 1px solid rgba(255, 255, 255, 0.15);
    z-index: 200;
    display: flex;
    flex-direction: column;
    transform: translateX(-100%);
    transition: transform 0.2s ease;
    will-change: transform;
  }

  .sidebar.open {
    transform: translateX(0);
  }

  :global([data-platform="linux"]) .sidebar {
    background: rgba(22, 22, 24, 0.92);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 12px 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .close-btn {
    width: 28px;
    height: 28px;
    border-radius: 4px;
    border: none;
    background: transparent;
    color: #aaa;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .close-btn:hover {
    color: #e0e0e0;
    background: rgba(255, 255, 255, 0.08);
  }

  .sidebar-title {
    font-size: 12px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.45);
    letter-spacing: 0.5px;
    text-transform: uppercase;
  }

  .tab-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .tab-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 6px;
    color: #aaa;
    font-size: 13px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .tab-item:hover {
    background: rgba(255, 255, 255, 0.06);
    color: #d0d0d0;
  }

  .tab-item.active {
    background: rgba(255, 255, 255, 0.1);
    color: #e0e0e0;
    font-weight: 600;
  }

  .tab-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .tab-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-meta {
    font-size: 11px;
    font-weight: 400;
    color: rgba(255, 255, 255, 0.3);
  }

  .tab-close-btn {
    width: 20px;
    height: 20px;
    border-radius: 4px;
    border: none;
    background: transparent;
    color: rgba(255, 255, 255, 0.2);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    opacity: 0;
    flex-shrink: 0;
  }

  .tab-item:hover .tab-close-btn {
    opacity: 1;
  }

  .tab-close-btn:hover {
    color: #e0e0e0;
    background: rgba(255, 255, 255, 0.1);
  }

  :global(.draft-dot) {
    color: #d97706;
    flex-shrink: 0;
    margin-top: 4px;
  }
</style>
