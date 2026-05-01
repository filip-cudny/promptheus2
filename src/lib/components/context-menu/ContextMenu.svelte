<script lang="ts">
  import { onMount, onDestroy, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { debug as logDebug } from "@tauri-apps/plugin-log";
  import type { MenuItem } from "$lib/types/menu";
  import {
    extractContextItems,
    extractLastInteractionData,
    groupBySection,
    type LastTextEntryRef,
  } from "./itemExtractors";
  import ContextSection from "./ContextSection.svelte";
  import LastInteractionSection from "./LastInteractionSection.svelte";
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
  import { PROMPTHEUS_PROVIDER_ID } from "$lib/services/shellToolbar";
  import ProviderMenuList from "$lib/components/provider-menu/ProviderMenuList.svelte";
  import { isExecuting, getExecutingSkillId } from "$lib/stores/execution.svelte";
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
  import { fetchSkillMetadata, buildSkillMetaEntries } from "$lib/services/skillMetadata.svelte";
  import { createWebviewProvidersStore } from "$lib/stores/webviewProviders.svelte";
  import { useModelsMenuData } from "$lib/stores/modelsMenuData.svelte";

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

  const providersStore = createWebviewProvidersStore();
  const models = useModelsMenuData(() => menu.items);

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

  let menuEl: HTMLDivElement | undefined = $state();
  let chatRowEl: HTMLElement | undefined = $state();
  let settingsToggleEl: HTMLElement | undefined = $state();
  let unlistenProviders: (() => void) | undefined;

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

  let sections = $derived(groupBySection(menu.items));

  let menuVisible = $derived(menu.visible);
  $effect(() => { if (!menuVisible) panels.closeAll(); });

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
    unlistenProviders = await providersStore.init();
    await blurClose.init();
  });

  onDestroy(() => {
    blurClose.destroy();
    menu.destroy();
    unlistenProviders?.();
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
            e.preventDefault();
            if (chatDisabled) return;
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
            providers={providersStore.providerEntries}
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
            models={models.modelsData?.models ?? []}
            sttModels={models.modelsData?.stt_models ?? []}
            defaultModelId={models.defaultModelId}
            reasoningEffort={models.reasoningEffort}
            sttModelId={models.sttModelId}
            quickActionCapabilities={models.quickActionCapabilities}
            preventDismiss={{ suppress: menu.suppressClose, resume: menu.resumeClose }}
            onModelSelect={models.setDefaultModel}
            onReasoningSelect={models.setReasoningEffort}
            onSttSelect={models.setSttModel}
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
            {@const metaEntries = buildSkillMetaEntries(skillId, models.modelNames)}
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
