<script lang="ts">
  import { onMount, onDestroy, tick, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { debug as logDebug } from "@tauri-apps/plugin-log";
  import { LogicalPosition, LogicalSize } from "@tauri-apps/api/dpi";
  import type { MenuItem } from "$lib/types/menu";
  import type { ContextItem } from "$lib/types/context";
  import ContextSection from "./ContextSection.svelte";
  import LastInteractionSection from "./LastInteractionSection.svelte";
  import ModelSelector from "$lib/components/ui/ModelSelector.svelte";
  import FloatingPanel from "$lib/components/ui/FloatingPanel.svelte";
  import MenuList from "$lib/components/ui/MenuList.svelte";
  import { ChevronRight, MessageSquare, MessageSquareShare, Mic, Square, X } from "lucide-svelte";
  import {
    focusOrOpenChat,
    openConversationDialog,
    openConversationDialogNewWindow,
  } from "$lib/services/conversationDialog";
  import {
    getWebviewProviders,
    type WebviewProvider,
  } from "$lib/services/aiWebview";
  import { PROMPTHEUS_PROVIDER_ID } from "$lib/services/shellToolbar";
  import { onSettingsChanged } from "$lib/services/events";
  import ProviderMenuList from "$lib/components/provider-menu/ProviderMenuList.svelte";
  import { isExecuting, getExecutingSkillId } from "$lib/stores/execution.svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { updateSurfaceModel, updateSurfaceReasoningEffort, setSpeechToTextModel } from "$lib/services/settings";
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
    isInBlurGrace,
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

  function isExecutingSkill(item: MenuItem): boolean {
    if (!isExecuting()) return false;
    const data = item.data as { skill_id: string } | null;
    return data?.skill_id === getExecutingSkillId();
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
    models: { id: string; display_name: string; model: string; provider: Provider }[];
    default_model_id: string | null;
    default_reasoning_effort: string | null;
    stt_models: { id: string; display_name: string; model: string; provider: Provider }[];
    speech_to_text_model_id: string | null;
  }

  function extractModelsData(item: MenuItem): ModelsMenuData | null {
    if (item.item_type !== "models") return null;
    return (item.data ?? null) as ModelsMenuData | null;
  }

  let modelsDefaultModelId = $state<string | null>(null);
  let modelsReasoningEffort = $state<string | null>(null);
  let sttModelId = $state<string | null>(null);

  let menuEl: HTMLDivElement | undefined = $state();
  let settingsOpen = $state(false);
  let settingsAnchorEl: HTMLElement | undefined = $state();
  let activeActionMenuId = $state("");
  let activeActionAnchorEl: HTMLElement | undefined = $state();
  let hoverEnabled = $state(false);
  let shiftHeld = $state(false);
  let chatProvidersOpen = $state(false);
  let chatRowEl: HTMLElement | undefined = $state();
  let webviewProviders = $state<WebviewProvider[]>([]);
  let unlistenSettings: (() => void) | undefined;

  let providerEntries = $derived<{ id: string; name: string; url?: string | null }[]>([
    { id: PROMPTHEUS_PROVIDER_ID, name: "Promptheus" },
    ...webviewProviders.map((p) => ({ id: p.id, name: p.name, url: p.url })),
  ]);

  async function refreshWebviewProviders() {
    try {
      webviewProviders = await getWebviewProviders();
    } catch (e) {
      console.error("getWebviewProviders failed", e);
    }
  }

  function closeChatProviders() {
    if (!chatProvidersOpen) return;
    chatProvidersOpen = false;
  }

  function openChatProviders(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (chatProvidersOpen) {
      closeChatProviders();
      return;
    }
    closePanels();
    chatProvidersOpen = true;
  }

  async function pickChatProvider(providerId: string) {
    closeChatProviders();
    await closeMenu();
    try {
      const arg = providerId === PROMPTHEUS_PROVIDER_ID ? undefined : providerId;
      await openConversationDialogNewWindow(undefined, arg);
    } catch (err) {
      console.error("openConversationDialogNewWindow failed", err);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Shift") shiftHeld = true;
    if (!menuVisible) return;

    switch (e.key) {
      case "Escape":
        e.preventDefault();
        if (settingsOpen || activeActionMenuId || chatProvidersOpen) {
          closePanels();
        } else {
          closeMenu();
        }
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
  let currentWindowPos = { x: 0, y: 0 };

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

    let height = menuEl.scrollHeight + 2;
    const win = getCurrentWebviewWindow();
    const wa = getWorkArea();
    let x = 0, y = 0;

    function positionFromHeight(h: number) {
      if (!wa) return;
      const anchorOffset = getSkillsSectionOffset();
      x = wa.cursorX;
      y = wa.cursorY - anchorOffset;
      const rightEdge = wa.workX + wa.workWidth;
      const bottomEdge = wa.workY + wa.workHeight;
      if (x + MENU_WIDTH > rightEdge) x = rightEdge - MENU_WIDTH;
      if (y + h > bottomEdge) y = bottomEdge - h;
      if (x < wa.workX) x = wa.workX;
      if (y < wa.workY) y = wa.workY;
    }

    positionFromHeight(height);
    hoverEnabled = false;
    await win.setSize(new LogicalSize(MENU_WIDTH, height));
    if (gen !== resizeGeneration || !isVisible()) return;
    if (wa) {
      currentWindowPos = { x, y };
      await win.setPosition(new LogicalPosition(x, y));
      if (gen !== resizeGeneration || !isVisible()) return;
    }
    await invoke("show_context_menu_panel");
    if (gen !== resizeGeneration || !isVisible()) return;

    const correctedHeight = menuEl.scrollHeight + 2;
    if (correctedHeight !== height) {
      height = correctedHeight;
      positionFromHeight(height);
      await win.setSize(new LogicalSize(MENU_WIDTH, height));
      if (gen !== resizeGeneration || !isVisible()) return;
      if (wa) {
        currentWindowPos = { x, y };
        await win.setPosition(new LogicalPosition(x, y));
        if (gen !== resizeGeneration || !isVisible()) return;
      }
    }

    await invoke("focus_context_menu");
    lastShownTrigger = getOpenTrigger();
    logDebug(`[ctx-menu] opened at (${x}, ${y}), size ${MENU_WIDTH}x${height}`);
  }

  function closeSettingsPanel() {
    if (settingsOpen) {
      logDebug("[ctx-menu] closing settings panel");
      settingsOpen = false;
      resumeClose();
    }
  }

  function closeActionMenu() {
    if (activeActionMenuId) {
      logDebug(`[ctx-menu] closing action menu: ${activeActionMenuId}`);
      activeActionMenuId = "";
      activeActionAnchorEl = undefined;
      resumeClose();
    }
  }

  function openActionMenu(e: MouseEvent, item: MenuItem, executingThis: boolean) {
    e.preventDefault();
    e.stopPropagation();
    if (executingThis) return;
    if (activeActionMenuId === item.id) {
      closeActionMenu();
      return;
    }
    closePanels();
    activeActionMenuId = item.id;
    activeActionAnchorEl = e.currentTarget as HTMLElement;
    suppressClose();
  }

  function closePanels() {
    closeSettingsPanel();
    closeActionMenu();
    closeChatProviders();
  }

  function handleMouseMove() {
    if (!hoverEnabled) hoverEnabled = true;
  }

  $effect(() => {
    void getOpenTrigger();
    if (menuVisible && menuItems.length > 0) {
      untrack(() => closePanels());
      resizeAndPositionWindow();
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

    return groups.filter((g) => g.sectionId !== "models");
  });

  let menuVisible = $derived(isVisible());
  $effect(() => { if (!menuVisible) closePanels(); });

  $effect(() => {
    const items = getItems();
    const modelsItem = items.find((i) => i.item_type === "models");
    if (modelsItem) {
      const data = extractModelsData(modelsItem);
      if (data) {
        modelsDefaultModelId = data.default_model_id;
        modelsReasoningEffort = data.default_reasoning_effort;
        sttModelId = data.speech_to_text_model_id;
      }
    }
  });
  let modelsData = $derived.by(() => {
    const modelsItem = getItems().find((i) => i.item_type === "models");
    return modelsItem ? extractModelsData(modelsItem) : null;
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

    await refreshWebviewProviders();
    unlistenSettings = await onSettingsChanged(refreshWebviewProviders);

    const win = getCurrentWebviewWindow();
    win.onFocusChanged(({ payload: focused }) => {
      if (!focused) {
        if (isSuppressed()) {
          resumeClose();
          return;
        }
        if (isInBlurGrace()) return;
        closeMenu();
      }
    });

  });

  onDestroy(() => {
    destroy();
    unlistenSettings?.();
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
        {@const chatDisabled = isRecording() && !chatRecording}
        <div
          class="chat-row"
          class:selected={chatRecording}
          role="menuitem"
          bind:this={chatRowEl}
          onmouseenter={() => { if (hoverEnabled) setSelectedIndex(-1); }}
          oncontextmenu={(e) => { if (!chatDisabled) openChatProviders(e); else e.preventDefault(); }}
        >
          <button
            class="chat-button"
            class:disabled={chatDisabled}
            onclick={async (e) => {
              if (chatDisabled) return;
              if (chatRecording || e.shiftKey) {
                await toggleChatRecording();
                return;
              }
              await closeMenu();
              await focusOrOpenChat();
            }}
          >
            {#if chatRecording}
              <Square size={ICON_SIZE.md} />
            {:else}
              <MessageSquare size={ICON_SIZE.md} />
            {/if}
            <span>Chat</span>
          </button>
        </div>
        <FloatingPanel visible={chatProvidersOpen} anchorEl={chatRowEl} flush onclose={closeChatProviders}>
          <ProviderMenuList
            providers={providerEntries}
            expand
            onSelect={(id) => { void pickChatProvider(id); }}
          />
        </FloatingPanel>
      {/if}
      {#if section.sectionId === "skills"}
        <div data-section="skills-anchor"></div>
      {/if}
      {#if section.sectionId === "settings"}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="menu-item-row"
          bind:this={settingsAnchorEl}
          onmouseenter={() => { if (hoverEnabled) setSelectedIndex(-1); }}
        >
          <button
            class="menu-item settings-toggle"
            role="menuitem"
            tabindex={-1}
            onclick={() => {
              if (settingsOpen) {
                closeSettingsPanel();
              } else {
                closePanels();
                logDebug("[ctx-menu] opening settings panel");
                settingsOpen = true;
                suppressClose();
              }
            }}
          >
            <span class="settings-chevron" class:expanded={settingsOpen}>
              <ChevronRight size={ICON_SIZE.sm} />
            </span>
            <span class="item-label">Settings</span>
          </button>
        </div>
        <FloatingPanel visible={settingsOpen} anchorEl={settingsAnchorEl} onclose={closeSettingsPanel}>
          {#if modelsData && modelsData.models.length > 0}
            <div class="panel-label">Quick action model</div>
            <div class="models-row" onmouseenter={() => { if (hoverEnabled) setSelectedIndex(-1); }}>
              <ModelSelector
                models={modelsData.models.map((m) => ({
                  id: m.id,
                  type: "text" as const,
                  model: m.model,
                  display_name: m.display_name,
                  provider: m.provider,
                  group: null,
                  api_key: null,
                  base_url: null,
                  parameters: null,
                  context_window_size: null,
                  api_mode: null,
                  store: true,
                }))}
                selectedModelId={modelsDefaultModelId}
                reasoningEffort={modelsReasoningEffort}
                onModelSelect={async (modelId) => {
                  modelsDefaultModelId = modelId;
                  await updateSurfaceModel("quick_actions", modelId);
                }}
                onReasoningSelect={async (effort) => {
                  modelsReasoningEffort = effort;
                  await updateSurfaceReasoningEffort("quick_actions", effort);
                }}
                preventDismiss={{ suppress: suppressClose, resume: resumeClose }}
              />
            </div>
          {/if}
          {#if modelsData && modelsData.stt_models.length > 0}
            <div class="panel-label">Speech-to-text model</div>
            <div class="models-row" onmouseenter={() => { if (hoverEnabled) setSelectedIndex(-1); }}>
              <ModelSelector
                models={modelsData.stt_models.map((m) => ({
                  id: m.id,
                  type: "stt" as const,
                  model: m.model,
                  display_name: m.display_name,
                  provider: m.provider,
                  group: null,
                  api_key: null,
                  base_url: null,
                  parameters: null,
                  context_window_size: null,
                  api_mode: null,
                  store: true,
                }))}
                selectedModelId={sttModelId}
                reasoningEffort={null}
                onModelSelect={async (modelId) => {
                  sttModelId = modelId;
                  await setSpeechToTextModel(modelId);
                }}
                onReasoningSelect={() => {}}
                preventDismiss={{ suppress: suppressClose, resume: resumeClose }}
              />
            </div>
          {/if}
        </FloatingPanel>
      {/if}
      {#each section.sectionId === "chat" || section.sectionId === "settings" ? [] : section.items as { item, globalIndex }}
        {@const contextItems = extractContextItems(item)}
        {@const lastInteractionData = extractLastInteractionData(item)}
        {#if contextItems}
          <ContextSection items={contextItems} />
        {:else if lastInteractionData !== null}
          <LastInteractionSection data={lastInteractionData} />
        {:else}
          {@const executingThis = item.item_type === "skill" && isExecutingSkill(item)}
          {@const recordingThis = item.item_type === "skill" && isRecordingThisSkill(item)}
          {@const micDisabled = executingThis || (!item.enabled && !recordingThis)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="menu-item-row"
            class:selected={globalIndex === currentSelectedIndex}
            onmouseenter={() => { if (hoverEnabled && item.enabled) setSelectedIndex(globalIndex); }}
            oncontextmenu={item.item_type === "skill"
              ? (e) => openActionMenu(e, item, executingThis)
              : undefined}
          >
            <button
              class="menu-item"
              class:disabled={!item.enabled}
              class:executing={executingThis}
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
                  {#if executingThis}
                    <span class="prompt-number executing"><X size={ICON_SIZE.sm} /></span>
                  {:else if recordingThis}
                    <span class="prompt-number executing"><Square size={ICON_SIZE.sm} /></span>
                  {:else}
                    <span class="prompt-number">{skillIndex + 1}.</span>
                  {/if}
                {/if}
              {/if}
              <span class="item-label">{item.label}</span>
            </button>
          </div>
          {#if item.item_type === "skill"}
            <FloatingPanel
              visible={activeActionMenuId === item.id}
              anchorEl={activeActionAnchorEl}
              flush
              onclose={closeActionMenu}
            >
              <MenuList role="menu" expand>
                <button
                  type="button"
                  role="menuitem"
                  class="menu-list-item"
                  onclick={() => {
                    closeActionMenu();
                    void openDialogForItem(globalIndex);
                  }}
                >
                  <MessageSquareShare size={ICON_SIZE.md} />
                  <span class="menu-list-label">Open in dialog</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  class="menu-list-item"
                  disabled={micDisabled}
                  onclick={() => {
                    closeActionMenu();
                    void startAlternativeExecution(globalIndex);
                  }}
                >
                  <Mic size={ICON_SIZE.md} />
                  <span class="menu-list-label">
                    {recordingThis ? "Stop recording" : "Run with transcription"}
                  </span>
                  <span class="menu-list-shortcut">⇧</span>
                </button>
                {#if item.tooltip}
                  <div class="menu-list-separator"></div>
                  <div class="menu-list-info">{item.tooltip}</div>
                {/if}
              </MenuList>
            </FloatingPanel>
          {/if}
        {/if}
      {/each}
    {/each}
    {#if menuItems.some((i) => i.item_type === "skill")}
      <div class="footer-hint" class:active={shiftHeld}>
        <span class="footer-hint-key">⇧</span>
        <span>voice input</span>
        <span class="footer-hint-sep">·</span>
        <span>right-click for actions</span>
      </div>
    {/if}
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
    flex-wrap: wrap;
    gap: 4px;
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

  .prompt-number.executing {
    color: #e0e0e0;
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .item-label {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .settings-toggle {
    gap: 4px;
  }

  .settings-chevron {
    display: flex;
    align-items: center;
    transition: transform 150ms ease;
    color: rgba(255, 255, 255, 0.4);
  }

  .settings-chevron.expanded {
    transform: rotate(90deg);
  }

  .panel-label {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.35);
    margin-bottom: 4px;
  }

  .footer-hint {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px 2px;
    margin-top: 2px;
    font-size: 10px;
    color: rgba(255, 255, 255, 0.25);
    user-select: none;
    transition: color 120ms ease;
  }

  .footer-hint.active {
    color: rgba(255, 255, 255, 0.55);
  }

  .footer-hint-key {
    font-weight: 600;
  }

  .footer-hint-sep {
    color: rgba(255, 255, 255, 0.18);
  }
</style>
