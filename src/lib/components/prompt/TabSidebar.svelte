<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { X, MessageSquare, MessagesSquare, Mic, Circle, EllipsisVertical, Pencil, Trash2 } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { getConversations, updateHistoryEntryTitle, deleteHistoryEntry } from "$lib/services/history";
  import { focusConversationInput } from "$lib/utils/conversationFocus";
  import type { HistoryEntry } from "$lib/types";
  import type { TabState } from "$lib/types/conversation";
  import type { createConversationStore } from "$lib/stores/conversation.svelte";

  const PAGE_SIZE = 30;

  let {
    store,
    open,
    onClose,
  }: {
    store: ReturnType<typeof createConversationStore>;
    open: boolean;
    onClose: () => void;
  } = $props();

  let conversations = $state<HistoryEntry[]>([]);
  let hasMore = $state(true);
  let loading = $state(false);
  let tabListEl: HTMLDivElement | undefined = $state();
  let unlistenHistoryChanged: UnlistenFn | undefined;

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
        const entry = conversations.find(e => e.id === tab.history_entry_id);
        if (entry) {
          result.push({ kind: "open", tab, entry });
        } else {
          result.push({ kind: "draft", tab });
        }
      } else {
        result.push({ kind: "draft", tab });
      }
    }

    for (const entry of conversations) {
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
    return entry.title ?? entry.skill_name ?? "New chat";
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
    if (editingItemId) return;
    if (item.kind === "history") {
      store.restoreFromHistory(item.entry.id, false);
    } else {
      store.switchTab(item.tab.tab_id);
    }
    onClose();
    focusConversationInput();
  }

  let menuOpenForId = $state<string | null>(null);
  let menuContainerEls = $state<Record<string, HTMLDivElement>>({});

  function toggleMenu(e: MouseEvent, item: SidebarItem) {
    e.stopPropagation();
    const id = itemId(item);
    menuOpenForId = menuOpenForId === id ? null : id;
  }

  function closeMenu() {
    menuOpenForId = null;
  }

  function handleWindowPointerDown(e: PointerEvent) {
    if (menuOpenForId) {
      const container = menuContainerEls[menuOpenForId];
      if (container && container.contains(e.target as Node)) return;
      closeMenu();
    }
  }

  let editingItemId = $state<string | null>(null);
  let editValue = $state("");
  let cancelled = $state(false);

  function startRename(item: SidebarItem) {
    closeMenu();
    editingItemId = itemId(item);
    editValue = itemTitle(item);
    cancelled = false;
  }

  function commitRename(item: SidebarItem) {
    if (cancelled) return;
    const trimmed = editValue.trim();
    if (trimmed && trimmed !== itemTitle(item)) {
      if (item.kind === "history") {
        updateHistoryEntryTitle(item.entry.id, trimmed).catch(() => {});
      } else {
        store.renameTab(item.tab.tab_id, trimmed);
      }
    }
    editingItemId = null;
  }

  function cancelRename() {
    cancelled = true;
    editingItemId = null;
  }

  function handleRenameKeydown(e: KeyboardEvent, item: SidebarItem) {
    if (e.key === "Enter") {
      e.preventDefault();
      (e.target as HTMLInputElement).blur();
    } else if (e.key === "Escape") {
      e.preventDefault();
      cancelRename();
    }
  }

  let confirmDeleteItem = $state<SidebarItem | null>(null);

  function startDelete(item: SidebarItem) {
    closeMenu();
    confirmDeleteItem = item;
  }

  async function confirmDelete() {
    if (!confirmDeleteItem) return;
    const item = confirmDeleteItem;
    confirmDeleteItem = null;

    if (item.kind === "open") {
      store.closeTab(item.tab.tab_id);
      await deleteHistoryEntry(item.entry.id).catch(() => {});
    } else if (item.kind === "history") {
      await deleteHistoryEntry(item.entry.id).catch(() => {});
    } else if (item.kind === "draft" && item.tab.history_entry_id) {
      const entryId = item.tab.history_entry_id;
      store.closeTab(item.tab.tab_id);
      await deleteHistoryEntry(entryId).catch(() => {});
    } else {
      store.closeTab(item.tab.tab_id);
    }
  }

  function cancelDelete() {
    confirmDeleteItem = null;
  }

  function handleConfirmKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      confirmDelete();
    } else if (e.key === "Escape") {
      e.preventDefault();
      cancelDelete();
    }
  }

  async function fetchPage(offset: number): Promise<HistoryEntry[]> {
    return getConversations(offset, PAGE_SIZE);
  }

  async function resetAndLoad() {
    loading = true;
    const page = await fetchPage(0);
    conversations = page;
    hasMore = page.length >= PAGE_SIZE;
    loading = false;
  }

  async function loadMore() {
    if (loading || !hasMore) return;
    loading = true;
    const page = await fetchPage(conversations.length);
    conversations = [...conversations, ...page];
    hasMore = page.length >= PAGE_SIZE;
    loading = false;
  }

  function handleScroll() {
    if (!tabListEl || !hasMore || loading) return;
    const { scrollTop, scrollHeight, clientHeight } = tabListEl;
    if (scrollHeight - scrollTop - clientHeight < 100) {
      loadMore();
    }
  }

  onMount(async () => {
    await resetAndLoad();
    unlistenHistoryChanged = await listen("history-changed", () => {
      resetAndLoad();
    });
  });

  onDestroy(() => {
    unlistenHistoryChanged?.();
  });

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

<svelte:window onpointerdown={handleWindowPointerDown} />

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

  <div class="tab-list" bind:this={tabListEl} onscroll={handleScroll}>
    {#each items as item (itemId(item))}
      {@const ts = itemTimestamp(item)}
      {@const id = itemId(item)}
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
          {#if editingItemId === id}
            <!-- svelte-ignore a11y_autofocus -->
            <input
              class="tab-name-input"
              type="text"
              bind:value={editValue}
              autofocus
              onclick={(e: MouseEvent) => e.stopPropagation()}
              onblur={() => commitRename(item)}
              onkeydown={(e: KeyboardEvent) => handleRenameKeydown(e, item)}
            />
          {:else}
            <span class="tab-name" title={itemTitle(item)}>{itemTitle(item)}</span>
          {/if}
          {#if ts}
            <span class="tab-meta">{ts}</span>
          {/if}
        </div>
        <div class="more-menu" bind:this={menuContainerEls[id]}>
          <button
            class="more-btn"
            onclick={(e: MouseEvent) => toggleMenu(e, item)}
          >
            <EllipsisVertical size={14} />
          </button>
          {#if menuOpenForId === id}
            <div class="menu-dropdown">
              <button class="menu-item" onclick={(e: MouseEvent) => { e.stopPropagation(); startRename(item); }}>
                <Pencil size={14} />
                <span>Rename</span>
              </button>
              <button class="menu-item destructive" onclick={(e: MouseEvent) => { e.stopPropagation(); startDelete(item); }}>
                <Trash2 size={14} />
                <span>Delete</span>
              </button>
            </div>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</aside>

{#if confirmDeleteItem}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="confirm-overlay" onclick={cancelDelete}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="confirm-dialog" onclick={(e: MouseEvent) => e.stopPropagation()} onkeydown={handleConfirmKeydown}>
      <p class="confirm-text">Delete this conversation?</p>
      <div class="confirm-actions">
        <button class="confirm-btn cancel" onclick={cancelDelete}>Cancel</button>
        <!-- svelte-ignore a11y_autofocus -->
        <button class="confirm-btn delete" onclick={confirmDelete} autofocus>
          <Trash2 size={14} />
          <span>Delete</span>
        </button>
      </div>
    </div>
  </div>
{/if}

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
    border-right: 1px solid var(--border-strong);
    z-index: var(--z-drawer);
    display: flex;
    flex-direction: column;
    transform: translateX(-100%);
    transition: transform var(--motion-slow) var(--ease-default);
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
    padding: var(--space-6) var(--space-6) var(--space-4);
    border-bottom: 1px solid var(--border-default);
  }

  .close-btn {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-md);
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-0);
  }

  .close-btn:hover {
    color: var(--text-primary);
    background: var(--surface-overlay);
  }

  .sidebar-title {
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-semibold);
    color: var(--text-muted);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  .tab-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .tab-item {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-5);
    border-radius: var(--radius-lg);
    color: var(--text-muted);
    font-size: var(--font-size-base);
    cursor: pointer;
    flex-shrink: 0;
  }

  .tab-item:hover {
    background: var(--surface-overlay-faint);
    color: var(--text-secondary);
  }

  .tab-item.active {
    background: var(--surface-overlay);
    color: var(--text-primary);
    font-weight: var(--font-weight-semibold);
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

  .tab-name-input {
    width: 100%;
    font-size: var(--font-size-base);
    font-family: inherit;
    color: var(--text-primary);
    background: var(--surface-overlay);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: var(--space-0) var(--space-2);
    outline: none;
    line-height: inherit;
  }

  .tab-name-input:focus {
    border-color: var(--border-strong);
  }

  .tab-meta {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-regular);
    color: var(--text-disabled);
  }

  .more-menu {
    position: relative;
    flex-shrink: 0;
  }

  .more-btn {
    width: 20px;
    height: 20px;
    border-radius: var(--radius-md);
    border: none;
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-0);
    opacity: 0;
  }

  .tab-item:hover .more-btn,
  .more-btn:focus {
    opacity: 1;
  }

  .more-btn:hover {
    color: var(--text-primary);
    background: var(--surface-overlay);
  }

  .menu-dropdown {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: var(--space-1);
    min-width: 120px;
    background: rgba(30, 30, 32, 0.98);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    padding: var(--space-2);
    z-index: 300;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4);
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: var(--font-size-md);
    cursor: pointer;
    border-radius: var(--radius-md);
    white-space: nowrap;
  }

  .menu-item:hover {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }

  .menu-item.destructive:hover {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }

  .confirm-overlay {
    position: fixed;
    inset: 0;
    background: var(--surface-scrim);
    z-index: 400;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .confirm-dialog {
    background: var(--surface-sunken);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-xl);
    padding: var(--space-8) var(--space-10);
    min-width: 240px;
    display: flex;
    flex-direction: column;
    gap: var(--space-7);
  }

  .confirm-text {
    margin: var(--space-0);
    font-size: var(--font-size-base);
    color: var(--text-primary);
  }

  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-4);
  }

  .confirm-btn {
    padding: 5px var(--space-7);
    border-radius: 5px;
    border: none;
    font-size: var(--font-size-md);
    cursor: pointer;
  }

  .confirm-btn.cancel {
    background: var(--surface-overlay);
    color: var(--text-secondary);
  }

  .confirm-btn.cancel:hover {
    background: rgba(255, 255, 255, 0.14);
  }

  .confirm-btn.delete {
    background: var(--surface-overlay-strong);
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 5px var(--space-5);
  }

  .confirm-btn.delete:hover {
    background: rgba(255, 255, 255, 0.14);
    color: var(--text-primary);
  }

  :global(.draft-dot) {
    color: var(--warning);
    flex-shrink: 0;
    margin-top: var(--space-2);
  }
</style>
