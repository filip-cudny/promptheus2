<script lang="ts">
  import { onMount, onDestroy, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { debug as logDebug } from "@tauri-apps/plugin-log";
  import type { MenuItem } from "$lib/types/menu";
  import type { ContextItem } from "$lib/types/context";
  import ContextSection from "./ContextSection.svelte";
  import LastInteractionSection from "./LastInteractionSection.svelte";
  import { prefetchCapabilities, getCachedCapabilities } from "$lib/stores/capabilities.svelte";
  import FloatingPanel from "$lib/components/ui/FloatingPanel.svelte";
  import {
    focusOrOpenChat,
    openConversationDialog,
    openConversationDialogNewWindow,
  } from "$lib/services/conversationDialog";
  import {
    clearContext,
    getContextText,
    setContextFromClipboard,
    appendContextFromClipboard,
    removeContextItem,
  } from "$lib/services/context";
  import { openContextEditor } from "$lib/services/contextEditor";
  import { copyHistoryContent } from "$lib/services/history";
  import { openHistoryDialog } from "$lib/services/historyDialog";
  import {
    getWebviewProviders,
    type WebviewProvider,
  } from "$lib/services/aiWebview";
  import { PROMPTHEUS_PROVIDER_ID } from "$lib/services/shellToolbar";
  import { onSettingsChanged } from "$lib/services/events";
  import ProviderMenuList from "$lib/components/provider-menu/ProviderMenuList.svelte";
  import { isExecuting, getExecutingSkillId } from "$lib/stores/execution.svelte";
  import { updateSurfaceModel, updateSurfaceReasoningEffort, setSpeechToTextModel } from "$lib/services/settings";
  import type { Provider } from "$lib/types";
  import { useContextMenu } from "$lib/stores/useContextMenu.svelte";
  import MenuShell from "./MenuShell.svelte";
  import { getWorkArea } from "$lib/stores/contextMenu.svelte";
  import MenuEmptyState from "./components/MenuEmptyState.svelte";
  import MenuSeparator from "./components/MenuSeparator.svelte";
  import MenuItemRow from "./components/MenuItemRow.svelte";
  import ChatRow from "./components/ChatRow.svelte";
  import SettingsToggleRow from "./components/SettingsToggleRow.svelte";
  import FooterHint from "./components/FooterHint.svelte";
  import SkillActionMenu from "./components/SkillActionMenu.svelte";
  import SettingsPanel from "./components/SettingsPanel.svelte";
  import { useFloatingPanelMutex } from "./drivers/useFloatingPanelMutex.svelte";
  import { useMenuKeyboard } from "./drivers/useMenuKeyboard.svelte";
  import { useMenuBlurClose } from "./drivers/useMenuBlurClose.svelte";
  import { useMenuPositioning } from "./drivers/useMenuPositioning.svelte";

  const menu = useContextMenu();

  const panels = useFloatingPanelMutex({
    onSettingsClose: menu.resumeClose,
    onActionMenuClose: menu.resumeClose,
  });

  const positioning = useMenuPositioning({
    getMenuEl: () => menuEl,
    isVisible: () => menu.visible,
    getWorkArea,
  });

  const keyboard = useMenuKeyboard({
    isVisible: () => menuVisible,
    hasOpenPanel: () => panels.hasAny,
    closePanels: panels.closeAll,
    closeMenu: menu.closeMenu,
    moveSelection: menu.moveSelection,
    executeSelected: menu.executeSelected,
    handleNumberInput: menu.handleNumberInput,
  });

  const blurClose = useMenuBlurClose({
    isInBlurGrace: menu.isInBlurGrace,
    isSuppressed: menu.isSuppressed,
    resumeClose: menu.resumeClose,
    closeMenu: menu.closeMenu,
  });

  function isRecordingThisSkill(item: MenuItem): boolean {
    if (!menu.recording) return false;
    const data = item.data as { skill_id: string } | null;
    return data?.skill_id === menu.recordingSkillId;
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
    models: { id: string; display_name: string; model: string; provider: Provider; group: string | null }[];
    default_model_id: string | null;
    default_reasoning_effort: string | null;
    stt_models: { id: string; display_name: string; model: string; provider: Provider; group: string | null }[];
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
  let chatRowEl: HTMLElement | undefined = $state();
  let settingsToggleEl: HTMLElement | undefined = $state();
  let skillMetadata = $state<Record<string, SkillMeta>>({});

  type SkillMeta = {
    model: string | null;
    parameters: Record<string, unknown> | null;
  };

  type MetaEntry = { key: string; value: string };

  function buildMetaEntries(
    meta: SkillMeta | undefined,
    modelNames: Map<string, string>,
  ): MetaEntry[] {
    if (!meta) return [];
    const out: MetaEntry[] = [];
    if (meta.model) {
      out.push({ key: "model", value: modelNames.get(meta.model) ?? meta.model });
    }
    if (meta.parameters) {
      for (const [k, v] of Object.entries(meta.parameters)) {
        if (v === null || v === undefined) continue;
        const value = typeof v === "object" ? JSON.stringify(v) : String(v);
        out.push({ key: k, value });
      }
    }
    return out;
  }

  async function fetchSkillMetadata(skillId: string) {
    if (skillId in skillMetadata) return;
    skillMetadata = { ...skillMetadata, [skillId]: { model: null, parameters: null } };
    try {
      const skill = await invoke<{
        model?: string | null;
        parameters?: Record<string, unknown> | null;
      }>("get_skill", { name: skillId });
      skillMetadata = {
        ...skillMetadata,
        [skillId]: {
          model: skill?.model ?? null,
          parameters: skill?.parameters ?? null,
        },
      };
    } catch (e) {
      logDebug(`get_skill failed for ${skillId}: ${e}`);
    }
  }

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

  async function pickChatProvider(providerId: string) {
    panels.closeChatProviders();
    await menu.closeMenu();
    try {
      const arg = providerId === PROMPTHEUS_PROVIDER_ID ? undefined : providerId;
      await openConversationDialogNewWindow(undefined, arg);
    } catch (err) {
      console.error("openConversationDialogNewWindow failed", err);
    }
  }

  function openActionMenu(e: MouseEvent, item: MenuItem, executingThis: boolean) {
    e.preventDefault();
    e.stopPropagation();
    if (executingThis) return;
    panels.openActionMenu(item.id, e.currentTarget as HTMLElement);
    if (panels.actionMenuId === item.id) {
      const skillId = (item.data as { skill_id?: string } | null)?.skill_id;
      if (skillId) void fetchSkillMetadata(skillId);
    }
  }

  $effect(() => {
    void menu.openTrigger;
    if (menuVisible && menuItems.length > 0) {
      untrack(() => panels.closeAll());
      void positioning.triggerReposition();
    }
  });

  let sections = $derived.by(() => {
    const allItems = menu.items;
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

  let menuVisible = $derived(menu.visible);
  $effect(() => { if (!menuVisible) panels.closeAll(); });

  $effect(() => {
    const items = menu.items;
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
    const modelsItem = menu.items.find((i) => i.item_type === "models");
    return modelsItem ? extractModelsData(modelsItem) : null;
  });
  let modelNames = $derived.by(() => {
    const map = new Map<string, string>();
    if (modelsData) {
      for (const m of modelsData.models) map.set(m.id, m.display_name);
      for (const m of modelsData.stt_models) map.set(m.id, m.display_name);
    }
    return map;
  });
  let quickActionModel = $derived.by(() => {
    if (!modelsData || !modelsDefaultModelId) return null;
    return modelsData.models.find((m) => m.id === modelsDefaultModelId) ?? null;
  });

  $effect(() => {
    if (quickActionModel) {
      prefetchCapabilities({
        id: quickActionModel.id,
        type: "text",
        model: quickActionModel.model,
        display_name: quickActionModel.display_name,
        provider: quickActionModel.provider,
        group: quickActionModel.group,
        api_key: null,
        base_url: null,
        parameters: null,
        context_window_size: null,
        api_mode: null,
        store: true,
      });
    }
  });

  let quickActionCapabilities = $derived(
    getCachedCapabilities(
      quickActionModel
        ? {
            id: quickActionModel.id,
            type: "text",
            model: quickActionModel.model,
            display_name: quickActionModel.display_name,
            provider: quickActionModel.provider,
            group: quickActionModel.group,
            api_key: null,
            base_url: null,
            parameters: null,
            context_window_size: null,
            api_mode: null,
            store: true,
          }
        : null,
    ),
  );

  let menuItems = $derived(menu.items);
  let allSkillItems = $derived(menu.allSkillItems);
  let currentSelectedIndex = $derived(menu.selectedIndex);

  $effect(() => {
    if (menuVisible && menuEl) {
      const _idx = currentSelectedIndex;
      const selected = menuEl.querySelector(".menu-item-row.selected");
      selected?.scrollIntoView({ block: "nearest" });
    }
  });

  function handleItemClick(index: number, e: MouseEvent) {
    menu.executeItem(index, e.shiftKey);
  }

  async function handleContextCopyAll() {
    const text = await getContextText();
    if (text) await navigator.clipboard.writeText(text);
  }

  async function handleOpenImagePreview(data: string, mediaType: string) {
    menu.suppressClose();
    await invoke("open_image_preview", { data, mediaType });
  }

  async function handleCopyHistoryContent(content: string) {
    await copyHistoryContent(content);
  }

  async function handleOpenLastInteraction(entry: LastTextEntryRef) {
    await menu.closeMenu();
    await openConversationDialog(entry.skill_id ?? "", entry.skill_name ?? "", entry.id, true);
  }

  async function handleOpenHistory() {
    await menu.closeMenu();
    await openHistoryDialog();
  }

  onMount(async () => {
    await menu.init();
    await refreshWebviewProviders();
    unlistenSettings = await onSettingsChanged(refreshWebviewProviders);
    await blurClose.init();
  });

  onDestroy(() => {
    blurClose.destroy();
    menu.destroy();
    unlistenSettings?.();
  });
</script>

<svelte:window onkeydown={keyboard.onkeydown} onkeyup={keyboard.onkeyup} />

<MenuShell bind:ref={menuEl} onmousemove={positioning.enableHover}>
  {#if menuItems.length === 0}
    <MenuEmptyState />
  {:else}
    {#each sections as section, sectionIdx}
      {#if sectionIdx > 0}
        <MenuSeparator />
      {/if}
      {#if section.sectionId === "chat"}
        {@const chatRecording = menu.chatRecording}
        {@const chatDisabled = menu.recording && !chatRecording}
        <ChatRow
          recording={chatRecording}
          disabled={chatDisabled}
          selected={chatRecording}
          bind:rowEl={chatRowEl}
          onhover={() => { if (positioning.hoverEnabled) menu.setSelectedIndex(-1); }}
          oncontextmenu={(e) => {
            if (chatDisabled) {
              e.preventDefault();
              return;
            }
            e.preventDefault();
            e.stopPropagation();
            panels.openChatProviders();
          }}
          onclick={async (e) => {
            if (chatDisabled) return;
            if (chatRecording || e.shiftKey) {
              await menu.toggleChatRecording();
              return;
            }
            await menu.closeMenu();
            await focusOrOpenChat();
          }}
        />
        <FloatingPanel
          visible={panels.chatProvidersOpen}
          anchorEl={chatRowEl}
          flush
          onclose={panels.closeChatProviders}
        >
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
        <SettingsToggleRow
          expanded={panels.settingsOpen}
          bind:anchorEl={settingsToggleEl}
          onhover={() => { if (positioning.hoverEnabled) menu.setSelectedIndex(-1); }}
          onclick={() => panels.openSettings(settingsToggleEl)}
        />
        <FloatingPanel
          visible={panels.settingsOpen}
          anchorEl={panels.settingsAnchor}
          onclose={panels.closeSettings}
        >
          <SettingsPanel
            models={modelsData?.models ?? []}
            sttModels={modelsData?.stt_models ?? []}
            defaultModelId={modelsDefaultModelId}
            reasoningEffort={modelsReasoningEffort}
            sttModelId={sttModelId}
            quickActionCapabilities={quickActionCapabilities}
            preventDismiss={{ suppress: menu.suppressClose, resume: menu.resumeClose }}
            onModelSelect={async (modelId) => {
              modelsDefaultModelId = modelId;
              await updateSurfaceModel("quick_actions", modelId);
            }}
            onReasoningSelect={async (effort) => {
              modelsReasoningEffort = effort;
              await updateSurfaceReasoningEffort("quick_actions", effort);
            }}
            onSttSelect={async (modelId) => {
              sttModelId = modelId;
              await setSpeechToTextModel(modelId);
            }}
            onHover={() => { if (positioning.hoverEnabled) menu.setSelectedIndex(-1); }}
          />
        </FloatingPanel>
      {/if}
      {#each section.sectionId === "chat" || section.sectionId === "settings" ? [] : section.items as { item, globalIndex }}
        {@const contextItems = extractContextItems(item)}
        {@const lastInteractionData = extractLastInteractionData(item)}
        {#if contextItems}
          <ContextSection
            items={contextItems}
            onReplaceFromClipboard={setContextFromClipboard}
            onAppendFromClipboard={appendContextFromClipboard}
            onOpenEditor={openContextEditor}
            onCopyAll={handleContextCopyAll}
            onClear={clearContext}
            onRemoveItem={removeContextItem}
            onOpenImagePreview={handleOpenImagePreview}
          />
        {:else if lastInteractionData !== null}
          <LastInteractionSection
            data={lastInteractionData}
            onCopyContent={handleCopyHistoryContent}
            onOpenLastInteraction={handleOpenLastInteraction}
            onOpenHistory={handleOpenHistory}
          />
        {:else}
          {@const executingThis = item.item_type === "skill" && isExecutingSkill(item)}
          {@const recordingThis = item.item_type === "skill" && isRecordingThisSkill(item)}
          {@const micDisabled = executingThis || (!item.enabled && !recordingThis)}
          {@const skillIndex = item.item_type === "skill" ? allSkillItems.indexOf(item) : -1}
          {@const promptNumber = skillIndex >= 0 ? skillIndex + 1 : null}
          <MenuItemRow
            selected={globalIndex === currentSelectedIndex}
            disabled={!item.enabled}
            executing={executingThis}
            recording={recordingThis}
            iconName={item.icon}
            promptNumber={promptNumber}
            label={item.label}
            onhover={() => { if (positioning.hoverEnabled && item.enabled) menu.setSelectedIndex(globalIndex); }}
            oncontextmenu={item.item_type === "skill"
              ? (e) => openActionMenu(e, item, executingThis)
              : undefined}
            onclick={(e) => handleItemClick(globalIndex, e)}
          />
          {#if item.item_type === "skill"}
            {@const skillId = (item.data as { skill_id?: string } | null)?.skill_id ?? ""}
            {@const metaEntries = buildMetaEntries(skillMetadata[skillId], modelNames)}
            <FloatingPanel
              visible={panels.actionMenuId === item.id}
              anchorEl={panels.actionMenuAnchor}
              flush
              onclose={panels.closeActionMenu}
            >
              <SkillActionMenu
                recording={recordingThis}
                {micDisabled}
                tooltip={item.tooltip}
                {metaEntries}
                onOpenInDialog={() => {
                  panels.closeActionMenu();
                  void menu.openDialogForItem(globalIndex);
                }}
                onAlternativeExecute={() => {
                  panels.closeActionMenu();
                  void menu.startAlternativeExecution(globalIndex);
                }}
              />
            </FloatingPanel>
          {/if}
        {/if}
      {/each}
    {/each}
    {#if menuItems.some((i) => i.item_type === "skill")}
      <FooterHint shiftHeld={keyboard.shiftHeld} />
    {/if}
  {/if}
</MenuShell>
