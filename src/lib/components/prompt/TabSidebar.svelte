<script lang="ts">
  import { X, Plus, MessageSquare } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
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

  function handleKeydown(e: KeyboardEvent) {
    if (open && e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

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
    {#each store.tabs as tab (tab.tab_id)}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        class="tab-item"
        class:active={tab.tab_id === store.activeTabId}
        onclick={() => handleTabClick(tab.tab_id)}
      >
        <MessageSquare size={ICON_SIZE.sm} />
        <span class="tab-name">{tab.tab_name}</span>
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

  <div class="sidebar-footer">
    <button class="new-tab-btn" onclick={() => store.addTab()}>
      <Plus size={ICON_SIZE.md} />
      <span>New Conversation</span>
    </button>
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
    background: #2a2a2a;
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

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 12px 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .sidebar-title {
    font-size: 12px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.5);
    text-transform: uppercase;
    letter-spacing: 0.5px;
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

  .tab-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .tab-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 6px;
    color: #aaa;
    font-size: 13px;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
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

  .tab-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
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

  .sidebar-footer {
    padding: 8px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
  }

  .new-tab-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: #aaa;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }

  .new-tab-btn:hover {
    background: rgba(255, 255, 255, 0.06);
    color: #e0e0e0;
  }
</style>
