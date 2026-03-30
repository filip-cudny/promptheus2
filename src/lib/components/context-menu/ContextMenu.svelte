<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { LogicalSize } from "@tauri-apps/api/dpi";
  import type { MenuItem } from "$lib/types/menu";
  import type { ContextItem } from "$lib/types/context";
  import ContextSection from "./ContextSection.svelte";
  import LastInteractionSection from "./LastInteractionSection.svelte";
  import { Info, MessageSquareShare, Mic, Square } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import {
    getItems,
    getSelectedIndex,
    setSelectedIndex,
    isVisible,
    isRecording,
    getRecordingPromptId,
    closeMenu,
    moveSelection,
    executeItem,
    executeSelected,
    startAlternativeExecution,
    handleNumberInput,
    clearNumberBuffer,
    getPromptItems,
    openDialogForItem,
    init,
    destroy,
  } from "$lib/stores/contextMenu.svelte";

  const SHIFTED_CHAR_TO_DIGIT: Record<string, string> = {
    "!": "1", "@": "2", "#": "3", "$": "4", "%": "5",
    "^": "6", "&": "7", "*": "8", "(": "9", ")": "0",
  };

  function isRecordingThisPrompt(item: MenuItem): boolean {
    if (!isRecording()) return false;
    const data = item.data as { prompt_id: string } | null;
    return data?.prompt_id === getRecordingPromptId();
  }

  type SectionGroup = {
    sectionId: string;
    startIndex: number;
    items: { item: MenuItem; globalIndex: number }[];
  };

  function extractContextItems(item: MenuItem): ContextItem[] | null {
    if (item.item_type !== "context") return null;
    const data = (item.data ?? {}) as { items?: ContextItem[] };
    return data.items ?? [];
  }

  interface LastInteractionChipData {
    content: string;
  }

  interface LastInteractionData {
    input: LastInteractionChipData | null;
    output: LastInteractionChipData | null;
    transcription: LastInteractionChipData | null;
  }

  function extractLastInteractionData(item: MenuItem): LastInteractionData | null {
    if (item.item_type !== "last_interaction") return null;
    return (item.data ?? null) as LastInteractionData | null;
  }

  let menuEl: HTMLDivElement | undefined = $state();
  let expandedDescriptionId = $state("");

  const MENU_WIDTH = 320;

  async function resizeWindowToContent() {
    await tick();
    if (!menuEl) return;
    const height = menuEl.scrollHeight;
    const win = getCurrentWebviewWindow();
    await win.setSize(new LogicalSize(MENU_WIDTH, height));
  }

  $effect(() => {
    void expandedDescriptionId;
    if (menuVisible && menuItems.length > 0) {
      resizeWindowToContent();
    }
  });

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
  $effect(() => { if (!menuVisible) expandedDescriptionId = ""; });
  let menuItems = $derived(getItems());
  let promptItems = $derived(getPromptItems());
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
        clearNumberBuffer();
        closeMenu();
        break;
      default: {
        if (e.key >= "0" && e.key <= "9") {
          e.preventDefault();
          handleNumberInput(e.key, e.shiftKey);
          return;
        }
        const mappedDigit = SHIFTED_CHAR_TO_DIGIT[e.key];
        if (mappedDigit) {
          e.preventDefault();
          handleNumberInput(mappedDigit, true);
        }
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
      if (!focused && !isRecording()) {
        closeMenu();
      }
    });
  });

  onDestroy(() => {
    destroy();
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="context-menu" role="menu" bind:this={menuEl}>
  {#if menuItems.length === 0}
    <div class="empty-state" role="menuitem">No items available</div>
  {:else}
    {#each sections as section, sectionIdx}
      {#if sectionIdx > 0}
        <div class="separator"></div>
      {/if}
      {#each section.items as { item, globalIndex }}
        {@const contextItems = extractContextItems(item)}
        {@const lastInteractionData = extractLastInteractionData(item)}
        {#if contextItems}
          <ContextSection items={contextItems} />
        {:else if lastInteractionData !== null}
          <LastInteractionSection data={lastInteractionData} />
        {:else}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="menu-item-row"
            class:selected={globalIndex === currentSelectedIndex}
            onmouseenter={() => { if (item.enabled) setSelectedIndex(globalIndex); }}
          >
            <button
              class="menu-item"
              class:disabled={!item.enabled}
              role="menuitem"
              aria-disabled={!item.enabled}
              tabindex={-1}
              onclick={(e) => handleItemClick(globalIndex, e)}
            >
              {#if item.icon === "square"}
                <span class="item-icon"><Square size={ICON_SIZE.sm} /></span>
              {:else if item.icon === "mic"}
                <span class="item-icon"><Mic size={ICON_SIZE.md} /></span>
              {/if}
              {#if item.item_type === "prompt"}
                {@const promptIndex = promptItems.indexOf(item)}
                {#if promptIndex >= 0}
                  <span class="prompt-number">{promptIndex + 1}.</span>
                {/if}
              {/if}
              <span class="item-label">{item.label}</span>
            </button>
            {#if item.item_type === "prompt" && item.tooltip}
              <button
                class="action-btn info-btn"
                onclick={(e) => { e.stopPropagation(); expandedDescriptionId = expandedDescriptionId === item.id ? "" : item.id; }}
              >
                <Info size={ICON_SIZE.sm} />
              </button>
            {/if}
            {#if item.item_type === "prompt"}
              {@const recordingThis = isRecordingThisPrompt(item)}
              {@const micDisabled = !item.enabled && !recordingThis}
              <button
                class="action-btn mic-btn"
                class:disabled={micDisabled}
                title={recordingThis ? "Stop recording" : "Voice input"}
                disabled={micDisabled}
                onclick={() => startAlternativeExecution(globalIndex)}
              >
                {#if recordingThis}
                  <Square size={ICON_SIZE.md} />
                {:else}
                  <Mic size={ICON_SIZE.md} />
                {/if}
              </button>
              <button
                class="action-btn dialog-btn"
                title="Open dialog"
                onclick={() => openDialogForItem(globalIndex)}
              >
                <MessageSquareShare size={ICON_SIZE.md} />
              </button>
            {/if}
          </div>
          {#if expandedDescriptionId === item.id && item.tooltip}
            <div class="description-row">{item.tooltip}</div>
          {/if}
        {/if}
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
    box-sizing: border-box;
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

  .menu-item-row {
    display: flex;
    align-items: center;
  }

  .menu-item-row.selected {
    background: rgba(255, 255, 255, 0.1);
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
    flex: 1;
    min-width: 0;
    box-sizing: border-box;
    border-radius: 0;
    outline: none;
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
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .prompt-number {
    flex-shrink: 0;
    min-width: 20px;
    text-align: right;
    color: rgba(255, 255, 255, 0.25);
    font-size: 12px;
    margin-left: -4px;
  }

  .item-label {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .action-btn {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.3);
    cursor: pointer;
  }

  .action-btn.dialog-btn {
    margin-right: 8px;
  }

  .action-btn:hover {
    background: rgba(255, 255, 255, 0.12);
    color: rgba(255, 255, 255, 0.8);
  }

  .action-btn.disabled {
    color: rgba(255, 255, 255, 0.15);
    cursor: default;
    pointer-events: none;
  }

  .info-btn {
    color: rgba(255, 255, 255, 0.15);
    width: 18px;
    height: 18px;
  }

  .description-row {
    padding: 2px 12px 6px 40px;
    color: rgba(255, 255, 255, 0.45);
    font-size: 12px;
    line-height: 1.3;
  }
</style>
