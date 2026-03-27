<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import type { MenuItem } from "$lib/types/menu";
  import {
    getItems,
    getSelectedIndex,
    setSelectedIndex,
    isVisible,
    closeMenu,
    moveSelection,
    executeItem,
    executeSelected,
    handleNumberInput,
    init,
    destroy,
  } from "$lib/stores/contextMenu.svelte";

  type SectionGroup = {
    sectionId: string;
    startIndex: number;
    items: { item: MenuItem; globalIndex: number }[];
  };

  let sections = $derived.by(() => {
    const allItems = getItems();
    const groups: SectionGroup[] = [];
    let currentSection: SectionGroup | null = null;

    for (let i = 0; i < allItems.length; i++) {
      const item = allItems[i];
      const sectionId = item.section_id ?? "default";

      if (!currentSection || currentSection.sectionId !== sectionId) {
        currentSection = { sectionId, startIndex: i, items: [] };
        groups.push(currentSection);
      }

      currentSection.items.push({ item, globalIndex: i });
    }

    return groups;
  });

  let menuVisible = $derived(isVisible());
  let menuItems = $derived(getItems());
  let currentSelectedIndex = $derived(getSelectedIndex());

  function handleKeydown(e: KeyboardEvent) {
    if (!menuVisible) return;

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        moveSelection(1);
        break;
      case "ArrowUp":
        e.preventDefault();
        moveSelection(-1);
        break;
      case "Enter":
        e.preventDefault();
        executeSelected(e.shiftKey);
        break;
      case "Escape":
        e.preventDefault();
        closeMenu();
        break;
      default:
        if (e.key >= "1" && e.key <= "9") {
          e.preventDefault();
          handleNumberInput(e.key);
        }
    }
  }

  function handleItemClick(index: number, e: MouseEvent) {
    executeItem(index, e.shiftKey);
  }

  onMount(async () => {
    await init();

    const win = getCurrentWebviewWindow();
    win.onFocusChanged(({ payload: focused }) => {
      if (!focused) {
        closeMenu();
      }
    });
  });

  onDestroy(() => {
    destroy();
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="context-menu" role="menu">
  {#if menuItems.length === 0}
    <div class="empty-state" role="menuitem">No items available</div>
  {:else}
    {#each sections as section, sectionIdx}
      {#if sectionIdx > 0}
        <div class="separator"></div>
      {/if}
      {#each section.items as { item, globalIndex }}
        <button
          class="menu-item"
          class:selected={globalIndex === currentSelectedIndex}
          class:disabled={!item.enabled}
          role="menuitem"
          aria-disabled={!item.enabled}
          tabindex={-1}
          onclick={(e) => handleItemClick(globalIndex, e)}
          onmouseenter={() => { if (item.enabled) setSelectedIndex(globalIndex); }}
        >
          {#if item.icon}
            <span class="item-icon">{item.icon}</span>
          {/if}
          <span class="item-label">{item.label}</span>
        </button>
      {/each}
    {/each}
  {/if}
</div>

<style>
  .context-menu {
    display: flex;
    flex-direction: column;
    background: #1e1e1e;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 4px 0;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    overflow-y: auto;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 13px;
    color: #e0e0e0;
  }

  .empty-state {
    padding: 12px 16px;
    color: rgba(255, 255, 255, 0.4);
    text-align: center;
    font-style: italic;
  }

  .separator {
    height: 1px;
    background: rgba(255, 255, 255, 0.1);
    margin: 4px 8px;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border: none;
    background: transparent;
    color: #e0e0e0;
    font: inherit;
    text-align: left;
    cursor: pointer;
    width: 100%;
    box-sizing: border-box;
    border-radius: 0;
    outline: none;
  }

  .menu-item.selected {
    background: rgba(255, 255, 255, 0.1);
  }

  .menu-item.disabled {
    color: rgba(255, 255, 255, 0.3);
    cursor: default;
  }

  .menu-item:not(.disabled):active {
    background: rgba(255, 255, 255, 0.15);
  }

  .item-icon {
    flex-shrink: 0;
    width: 16px;
    text-align: center;
  }

  .item-label {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
