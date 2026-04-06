<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { debug as logDebug } from "@tauri-apps/plugin-log";
  import { LogicalPosition, LogicalSize } from "@tauri-apps/api/dpi";
  import type { MenuItem } from "$lib/types/menu";
  import type { ContextItem } from "$lib/types/context";
  import ContextSection from "./ContextSection.svelte";
  import LastInteractionSection from "./LastInteractionSection.svelte";
  import ModelSelector from "$lib/components/ui/ModelSelector.svelte";
  import { Info, MessageSquare, MessageSquareShare, Mic, Square } from "lucide-svelte";
  import { openConversationDialog } from "$lib/services/conversationDialog";
  import { isExecuting } from "$lib/stores/execution.svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { updateSetting, updateModelReasoningEffort } from "$lib/services/settings";
  import type { ModelConfig, Provider } from "$lib/types";
  import {
    getItems,
    getSelectedIndex,
    setSelectedIndex,
    isVisible,
    getOpenTrigger,
    isRecording,
    getRecordingSkillId,
    closeMenu,
    suppressClose,
    isSuppressed,
    resumeClose,
    moveSelection,
    executeItem,
    executeSelected,
    startAlternativeExecution,
    handleNumberInput,
    clearNumberBuffer,
    getAllSkillItems,
    getSkillItems,
    isRecordingChat,
    toggleChatRecording,
    openDialogForItem,
    getWorkArea,
    init,
    destroy,
  } from "$lib/stores/contextMenu.svelte";

  const SHIFTED_CHAR_TO_DIGIT: Record<string, string> = {
    "!": "1", "@": "2", "#": "3", "$": "4", "%": "5",
    "^": "6", "&": "7", "*": "8", "(": "9", ")": "0",
  };

  function isRecordingThisSkill(item: MenuItem): boolean {
    if (!isRecording()) return false;
    const data = item.data as { skill_id: string } | null;
    return data?.skill_id === getRecordingSkillId();
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
    preview: string;
  }

  interface LastTextEntryRef {
    id: string;
    skill_id: string | null;
    skill_name: string | null;
  }

  interface LastInteractionData {
    input: LastInteractionChipData | null;
    output: LastInteractionChipData | null;
    transcription: LastInteractionChipData | null;
    last_text_entry: LastTextEntryRef | null;
  }

  function extractLastInteractionData(item: MenuItem): LastInteractionData | null {
    if (item.item_type !== "last_interaction") return null;
    return (item.data ?? null) as LastInteractionData | null;
  }

  interface ModelsMenuData {
    models: { id: string; display_name: string; model: string; provider: Provider; reasoning_effort: string | null }[];
    default_model_id: string | null;
    default_reasoning_effort: string | null;
  }

  function extractModelsData(item: MenuItem): ModelsMenuData | null {
    if (item.item_type !== "models") return null;
    return (item.data ?? null) as ModelsMenuData | null;
  }

  let modelsDefaultModelId = $state<string | null>(null);
  let modelsReasoningEffort = $state<string | null>(null);

  let menuEl: HTMLDivElement | undefined = $state();
  let expandedDescriptionId = $state("");
  let hoverEnabled = $state(false);
  let shiftHeld = $state(false);

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Shift") shiftHeld = true;
    if (!menuVisible) return;

    switch (e.key) {
      case "Escape":
        e.preventDefault();
        closeMenu();
        break;
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

  function handleKeyup(e: KeyboardEvent) {
    if (e.key === "Shift") shiftHeld = false;
  }

  const MENU_WIDTH = 320;
  let resizeGeneration = 0;
  let lastShownTrigger = 0;

  function getSkillsSectionOffset(): number {
    if (!menuEl) return 0;
    const anchor = menuEl.querySelector("[data-section='skills-anchor']");
    if (!anchor) return 0;
    return (anchor as HTMLElement).offsetTop;
  }

  async function resizeAndPositionWindow() {
    const gen = ++resizeGeneration;
    await tick();
    if (gen !== resizeGeneration) return;
    if (!menuEl || !isVisible()) return;

    const height = menuEl.scrollHeight + 2;
    const win = getCurrentWebviewWindow();
    const wa = getWorkArea();
    let x = 0, y = 0;
    if (wa) {
      const anchorOffset = getSkillsSectionOffset();
      x = wa.cursorX;
      y = wa.cursorY - anchorOffset;

      const rightEdge = wa.workX + wa.workWidth;
      const bottomEdge = wa.workY + wa.workHeight;
      if (x + MENU_WIDTH > rightEdge) x = rightEdge - MENU_WIDTH;
      if (y + height > bottomEdge) y = bottomEdge - height;
      if (x < wa.workX) x = wa.workX;
      if (y < wa.workY) y = wa.workY;
    }

    hoverEnabled = false;
    suppressClose();
    await win.hide();
    if (gen !== resizeGeneration || !isVisible()) { resumeClose(); return; }
    await win.setSize(new LogicalSize(MENU_WIDTH, height));
    if (gen !== resizeGeneration || !isVisible()) { resumeClose(); return; }
    if (wa) {
      await win.setPosition(new LogicalPosition(x, y));
      if (gen !== resizeGeneration || !isVisible()) { resumeClose(); return; }
    }
    await win.show();
    if (gen !== resizeGeneration || !isVisible()) { resumeClose(); return; }
    await invoke("focus_context_menu");
    resumeClose();
    lastShownTrigger = getOpenTrigger();
    logDebug(`[ctx-menu] opened at (${x}, ${y}), size ${MENU_WIDTH}x${height}`);
  }

  async function resizeWindow() {
    if (lastShownTrigger !== getOpenTrigger()) return;
    const gen = ++resizeGeneration;
    await tick();
    if (gen !== resizeGeneration) return;
    if (!menuEl || !isVisible()) return;
    const height = menuEl.scrollHeight + 2;
    const win = getCurrentWebviewWindow();
    await win.setSize(new LogicalSize(MENU_WIDTH, height));
  }

  function handleMouseMove() {
    if (!hoverEnabled) hoverEnabled = true;
  }

  $effect(() => {
    void getOpenTrigger();
    if (menuVisible && menuItems.length > 0) {
      resizeAndPositionWindow();
    }
  });

  $effect(() => {
    void expandedDescriptionId;
    void menuItems;
    if (menuVisible && menuItems.length > 0) {
      resizeWindow();
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

  $effect(() => {
    const items = getItems();
    const modelsItem = items.find((i) => i.item_type === "models");
    if (modelsItem) {
      const data = extractModelsData(modelsItem);
      if (data) {
        modelsDefaultModelId = data.default_model_id;
        modelsReasoningEffort = data.default_reasoning_effort;
      }
    }
  });
  let menuItems = $derived(getItems());
  let allSkillItems = $derived(getAllSkillItems());
  let skillItems = $derived(getSkillItems());
  let currentSelectedIndex = $derived(getSelectedIndex());

  $effect(() => {
    if (menuVisible && menuEl) {
      const _idx = currentSelectedIndex;
      const selected = menuEl.querySelector(".menu-item-row.selected");
      selected?.scrollIntoView({ block: "nearest" });
    }
  });

  function handleItemClick(index: number, e: MouseEvent) {
    executeItem(index, e.shiftKey);
  }

  onMount(async () => {
    await init();

    const win = getCurrentWebviewWindow();
    win.onFocusChanged(({ payload: focused }) => {
      if (!focused && !isRecording()) {
        if (isSuppressed()) {
          resumeClose();
          return;
        }
        closeMenu();
      }
    });
  });

  onDestroy(() => {
    destroy();
  });
</script>

<svelte:window onkeydown={handleKeydown} onkeyup={handleKeyup} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="context-menu" role="menu" bind:this={menuEl} onmousemove={handleMouseMove}>
  {#if menuItems.length === 0}
    <div class="empty-state" role="menuitem">No items available</div>
  {:else}
    {#each sections as section, sectionIdx}
      {#if sectionIdx > 0}
        <div class="separator"></div>
      {/if}
      {#if section.sectionId === "chat"}
        {@const chatRecording = isRecordingChat()}
        {@const chatDisabled = (isRecording() && !chatRecording) || isExecuting()}
        <div class="chat-row" class:selected={chatRecording} role="menuitem" onmouseenter={() => { if (hoverEnabled) setSelectedIndex(-1); }}>
          <button
            class="chat-button"
            class:disabled={chatDisabled}
            onclick={async () => {
              if (chatDisabled) return;
              if (chatRecording) {
                await toggleChatRecording();
              } else {
                await closeMenu();
                await openConversationDialog("", "Chat");
              }
            }}
          >
            <MessageSquare size={ICON_SIZE.md} />
            <span>Chat</span>
          </button>
          <button
            class="action-btn mic-btn chat-mic-btn"
            class:disabled={chatDisabled}
            class:shift-accent={shiftHeld && !chatDisabled && !chatRecording}
            title={chatRecording ? "Stop recording" : "Voice input for chat"}
            disabled={chatDisabled}
            onclick={() => toggleChatRecording()}
          >
            {#if chatRecording}
              <Square size={ICON_SIZE.md} />
            {:else}
              <Mic size={ICON_SIZE.md} />
            {/if}
          </button>
        </div>
      {/if}
      {#if section.sectionId === "models"}
        {@const modelsItem = section.items[0]?.item}
        {@const modelsData = modelsItem ? extractModelsData(modelsItem) : null}
        {#if modelsData && modelsData.models.length > 0}
          <div class="models-row" onmouseenter={() => { if (hoverEnabled) setSelectedIndex(-1); }}>
            <ModelSelector
              models={modelsData.models.map((m) => ({
                id: m.id,
                model: m.model,
                display_name: m.display_name,
                provider: m.provider,
                api_key_source: "env" as const,
                api_key_env: null,
                api_key: null,
                base_url: null,
                parameters: m.reasoning_effort ? { temperature: null, max_tokens: null, top_p: null, frequency_penalty: null, presence_penalty: null, reasoning_effort: m.reasoning_effort } : null,
                context_window_size: null,
                enabled_tools: [],
              }))}
              selectedModelId={modelsDefaultModelId}
              reasoningEffort={modelsReasoningEffort}
              onModelSelect={async (modelId) => {
                modelsDefaultModelId = modelId;
                const model = modelsData.models.find((m) => m.id === modelId);
                modelsReasoningEffort = model?.reasoning_effort ?? null;
                await updateSetting("default_model", modelId);
              }}
              onReasoningSelect={async (effort) => {
                modelsReasoningEffort = effort;
                if (modelsDefaultModelId) {
                  await updateModelReasoningEffort(modelsDefaultModelId, effort);
                }
              }}
              preventDismiss={{ suppress: suppressClose, resume: resumeClose }}
              onDropdownToggle={() => resizeWindow()}
            />
          </div>
        {/if}
      {/if}
      {#if section.sectionId === "skills"}
        <div data-section="skills-anchor"></div>
      {/if}
      {#each section.sectionId === "chat" || section.sectionId === "models" ? [] : section.items as { item, globalIndex }}
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
            onmouseenter={() => { if (hoverEnabled && item.enabled) setSelectedIndex(globalIndex); }}
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
              {#if item.item_type === "skill"}
                {@const skillIndex = allSkillItems.indexOf(item)}
                {#if skillIndex >= 0}
                  <span class="prompt-number">{skillIndex + 1}.</span>
                {/if}
              {/if}
              <span class="item-label">{item.label}</span>
            </button>
            {#if item.item_type === "skill"}
              {@const recordingThis = isRecordingThisSkill(item)}
              {@const micDisabled = !item.enabled && !recordingThis}
              {#if item.tooltip}
                <button
                  class="action-btn info-btn"
                  onclick={(e) => { e.stopPropagation(); expandedDescriptionId = expandedDescriptionId === item.id ? "" : item.id; }}
                >
                  <Info size={ICON_SIZE.sm} />
                </button>
              {/if}
              <button
                class="action-btn mic-btn"
                class:disabled={micDisabled}
                class:shift-accent={shiftHeld && !micDisabled && !recordingThis}
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
    overflow-y: hidden;
  }

  .empty-state {
    padding: 12px 16px;
    color: rgba(255, 255, 255, 0.4);
    text-align: center;
    font-style: italic;
  }

  .models-row {
    display: flex;
    align-items: center;
    padding: 4px 12px;
  }

  .chat-row {
    display: flex;
    align-items: center;
  }

  .chat-row:hover,
  .chat-row.selected {
    background: rgba(255, 255, 255, 0.1);
  }

  .chat-row:active {
    background: rgba(255, 255, 255, 0.15);
  }

  .chat-button {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
    padding: 6px 12px;
    border: none;
    background: transparent;
    color: #e0e0e0;
    font: inherit;
    text-align: left;
    cursor: pointer;
    box-sizing: border-box;
    outline: none;
  }

  .chat-button.disabled {
    color: rgba(255, 255, 255, 0.3);
    cursor: default;
  }

  .chat-mic-btn {
    margin-right: 30px;
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

  .menu-item-row.selected:active {
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

  .action-btn.hidden-placeholder {
    visibility: hidden;
    pointer-events: none;
  }

  .mic-btn.shift-accent {
    color: rgba(255, 255, 255, 0.6);
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
