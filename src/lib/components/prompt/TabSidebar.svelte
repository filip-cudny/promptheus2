<script lang="ts">
  import { X, MessageSquare, MessagesSquare } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import type { TabState } from "$lib/types";
  import type { createConversationStore } from "$lib/stores/conversation.svelte";

  let {
    store,
    open,
    onClose,
  }: {
    store: ReturnType<typeof createConversationStore>;
    open: boolean;
    onClose: () => void;
  } = $props();

  const showCloseButtons = $derived(store.tabs.length > 1);

  function handleTabClick(tabId: string) {
    store.switchTab(tabId);
    onClose();
  }

  function handleCloseTab(e: MouseEvent, tabId: string) {
    e.stopPropagation();
    store.closeTab(tabId);
  }

  function tabTurnCount(tab: TabState): number {
    const path = tab.tree.current_path;
    let count = 0;
    for (const nodeId of path) {
      const node = tab.tree.nodes.get(nodeId);
      if (node?.role === "user") count++;
    }
    return count;
  }

  function tabTimestamp(tab: TabState): string | null {
    const path = tab.tree.current_path;
    if (path.length === 0) return null;
    const lastNode = tab.tree.nodes.get(path[path.length - 1]);
    if (!lastNode) return null;
    return formatTimestamp(lastNode.timestamp);
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
    {#each [...store.tabs].reverse() as tab (tab.tab_id)}
      {@const turns = tabTurnCount(tab)}
      {@const ts = tabTimestamp(tab)}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        class="tab-item"
        class:active={tab.tab_id === store.activeTabId}
        onclick={() => handleTabClick(tab.tab_id)}
      >
        {#if turns > 1}
          <MessagesSquare size={ICON_SIZE.sm} />
        {:else}
          <MessageSquare size={ICON_SIZE.sm} />
        {/if}
        <div class="tab-body">
          <span class="tab-name">{tab.tab_name}</span>
          {#if ts}
            <span class="tab-meta">
              {ts}{#if turns > 0}&nbsp;&middot;&nbsp;{turns} {turns === 1 ? "turn" : "turns"}{/if}
            </span>
          {/if}
        </div>
        {#if showCloseButtons}
          <button
            class="tab-close"
            onclick={(e: MouseEvent) => handleCloseTab(e, tab.tab_id)}
          >
            <X size={ICON_SIZE.sm} />
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

  .tab-close {
    flex-shrink: 0;
    width: 20px;
    height: 20px;
    border-radius: 4px;
    border: none;
    background: transparent;
    color: #666;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    opacity: 0;
  }

  .tab-item:hover .tab-close {
    opacity: 1;
  }

  .tab-close:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #e0e0e0;
  }
</style>
