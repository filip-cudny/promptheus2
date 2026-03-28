<script lang="ts">
  let {
    tabs,
    activeTabId,
    onSelect,
    onClose,
  }: {
    tabs: { id: string; name: string }[];
    activeTabId: string;
    onSelect: (tabId: string) => void;
    onClose: (tabId: string) => void;
  } = $props();

  const showCloseButtons = $derived(tabs.length > 1);
</script>

{#if tabs.length > 1}
  <div class="tab-bar">
    {#each tabs as tab (tab.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        class="tab"
        class:active={tab.id === activeTabId}
        onclick={() => onSelect(tab.id)}
      >
        <span class="tab-name">{tab.name}</span>
        {#if showCloseButtons}
          <button
            class="tab-close"
            onclick={(e: MouseEvent) => { e.stopPropagation(); onClose(tab.id); }}
            title="Close tab"
          >
            &times;
          </button>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .tab-bar {
    display: flex;
    gap: 2px;
    overflow-x: auto;
    scrollbar-width: thin;
    scrollbar-color: #555 transparent;
  }

  .tab-bar::-webkit-scrollbar {
    height: 4px;
  }

  .tab-bar::-webkit-scrollbar-thumb {
    background: #555;
    border-radius: 2px;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.04);
    color: #aaa;
    font-size: 11px;
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .tab:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .tab.active {
    background: rgba(255, 255, 255, 0.12);
    color: #e0e0e0;
    font-weight: 600;
    border-color: rgba(255, 255, 255, 0.2);
  }

  .tab-name {
    pointer-events: none;
  }

  .tab-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    border: none;
    border-radius: 2px;
    background: transparent;
    color: #888;
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
  }

  .tab-close:hover {
    background: rgba(255, 255, 255, 0.15);
    color: #e0e0e0;
  }
</style>
