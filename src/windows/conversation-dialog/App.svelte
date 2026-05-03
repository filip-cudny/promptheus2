<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { createConversationStore } from "$lib/stores/conversation.svelte";
  import { hasContext } from "$lib/services/context";
  import { getSettings } from "$lib/services/settings";
  import { getContextWindowSize } from "$lib/utils/contextWindow";
  import { getSkillsStore } from "$lib/stores/skills.svelte";
  import ChatPalette from "$lib/components/features/conversation-dialog/ChatPalette.svelte";
  import ChatTopButtons from "$lib/components/features/conversation-dialog/ChatTopButtons.svelte";
  import ConversationArea from "$lib/components/features/conversation-dialog/ConversationArea.svelte";
  import InputArea from "$lib/components/features/conversation-dialog/InputArea.svelte";
  import TabSidebar from "$lib/components/features/conversation-dialog/TabSidebar.svelte";
  import { useConversationDialogIpc } from "$lib/components/features/conversation-dialog/drivers/useConversationDialogIpc.svelte";
  import { openPalette, reloadActiveInHost } from "$lib/services/shellToolbar";
  import { SHORTCUTS, matches } from "$lib/shortcuts";
  import { focusConversationInput } from "$lib/utils/conversationFocus";
  import { initTheme } from "$lib/stores/theme.svelte";
  import type { ModelConfig } from "$lib/types";

  interface DialogInitParams {
    skill_id: string;
    skill_name: string;
    skill_model: string | null;
    history_entry_id: string | null;
    last_interaction_only: boolean;
    initial_input: string | null;
    auto_send_input: boolean;
    new_chat: boolean;
  }

  const skillsStore = getSkillsStore();
  const HOST_LABEL = getCurrentWindow().label;
  const SELF_TARGET = getCurrentWebview().label;

  const store = createConversationStore("", "");

  let sidebarOpen = $state(false);
  let contextVisible = $state(false);
  let contextDisabled = $state(false);
  let contextInitialCollapsed = $state(false);
  let models = $state<ModelConfig[]>([]);
  let defaultModelId = $state<string | null>(null);

  let chatPaletteOpen = $state(false);
  let prevChatPaletteOpen = false;

  $effect(() => {
    const isOpen = chatPaletteOpen;
    if (prevChatPaletteOpen && !isOpen) {
      focusConversationInput();
    }
    prevChatPaletteOpen = isOpen;
  });

  let contextWindowSize = $derived.by(() => {
    const activeModelId = store.modelId ?? defaultModelId;
    const activeModel = models.find((m) => m.id === activeModelId);
    if (!activeModel) return 0;
    return getContextWindowSize(activeModel.model, activeModel.context_window_size);
  });

  async function handleGlobalKeydown(e: KeyboardEvent) {
    if (matches(e, SHORTCUTS.reloadActive)) {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (chatPaletteOpen) chatPaletteOpen = false;
      await reloadActiveProvider();
      return;
    }

    if (chatPaletteOpen) return;

    if (matches(e, SHORTCUTS.openPalette)) {
      e.preventDefault();
      e.stopPropagation();
      try {
        await openPalette(HOST_LABEL);
      } catch (err) {
        console.error("open_palette failed", err);
      }
      return;
    }

    if (matches(e, SHORTCUTS.openChatPalette)) {
      e.preventDefault();
      e.stopPropagation();
      chatPaletteOpen = true;
      return;
    }

    if (e.key === "Escape" && store.isExecuting) {
      e.preventDefault();
      store.abortExecution();
    }
  }

  function handleChatPaletteNewChat() {
    chatPaletteOpen = false;
    store.addTab();
  }

  async function handleChatPaletteOpenConversation(entryId: string) {
    chatPaletteOpen = false;
    await store.restoreFromHistory(entryId);
  }

  async function reloadActiveProvider() {
    try {
      await reloadActiveInHost(HOST_LABEL);
    } catch (err) {
      console.error("reload_active_in_host failed", err);
    }
  }

  async function autoShowContextIfNeeded() {
    if (contextDisabled || contextVisible) return;
    if (await hasContext()) {
      contextInitialCollapsed = true;
      contextVisible = true;
    }
  }

  function handleVoiceInput(skillId: string, text: string, autoSend: boolean) {
    const currentTab = store.tabs.find((t) => t.tab_id === store.activeTabId);
    if (currentTab && currentTab.tree.current_path.length > 0) {
      store.addTab();
    }
    const inputText = skillId ? `/${skillId} ${text}` : text;
    store.updateInputText(inputText);
    if (autoSend) store.sendMessage();
  }

  async function applyInitParams(p: DialogInitParams) {
    if (p.new_chat) {
      store.addTab();
    } else if (p.history_entry_id) {
      await store.restoreFromHistory(p.history_entry_id, p.last_interaction_only);
    } else if (p.initial_input) {
      handleVoiceInput(p.skill_id, p.initial_input, p.auto_send_input);
    } else if (p.skill_id) {
      store.openForSkill(p.skill_id, p.skill_name, p.skill_model);
    }
  }

  async function loadModelInfo() {
    try {
      const settings = await getSettings();
      models = settings.models.filter((m) => m.type === "text");
      defaultModelId = settings.surfaces.chat.generation.model_id ?? null;
    } catch {}
  }

  const ipc = useConversationDialogIpc({
    selfTarget: SELF_TARGET,
    onRestoreHistory: (entryId, lastInteractionOnly) => {
      store.restoreFromHistory(entryId, lastInteractionOnly);
    },
    onVoiceInput: (text, autoSend) => handleVoiceInput("", text, autoSend),
    onOpenForSkill: (skillId, skillName, skillModel) =>
      store.openForSkill(skillId, skillName, skillModel),
    onNewConversation: () => store.addTab(),
    onActiveProviderCleared: () => focusConversationInput(),
    onMenuReloadActive: () => {
      if (chatPaletteOpen) chatPaletteOpen = false;
      reloadActiveProvider();
    },
    onContextChanged: () => autoShowContextIfNeeded(),
  });

  onMount(async () => {
    initTheme();
    window.addEventListener("keydown", handleGlobalKeydown);
    skillsStore.init();
    await store.initFromSettings();
    loadModelInfo();

    const reconnected = await store.tryReconnect();

    const initParams = await invoke<DialogInitParams | null>("get_dialog_init_params");
    if (!reconnected && initParams) {
      await applyInitParams(initParams);
    }

    await ipc.init();
    await autoShowContextIfNeeded();
  });

  function handleContextAutoShow() {
    if (!contextDisabled) {
      contextInitialCollapsed = !contextVisible;
      contextVisible = true;
    }
  }

  function toggleContext() {
    if (contextVisible) closeContext();
    else contextVisible = true;
  }

  function closeContext() {
    contextVisible = false;
    store.updateContextText("");
    store.updateContextImages([]);
  }

  async function handleSendAndCopy() {
    const currentWindow = getCurrentWindow();
    const { success, result } = await store.sendMessage();
    if (success && result) {
      await navigator.clipboard.writeText(result);
      await currentWindow.close();
    }
  }

  onDestroy(() => {
    window.removeEventListener("keydown", handleGlobalKeydown);
    ipc.destroy();
    store.destroy();
  });
</script>

<div class="dialog-shell">
  <ChatTopButtons
    {sidebarOpen}
    onToggleSidebar={() => (sidebarOpen = !sidebarOpen)}
    onNewChat={() => store.addTab()}
  />
  <ConversationArea {store} />
  <InputArea
    {store}
    {models}
    {contextVisible}
    {contextDisabled}
    {contextInitialCollapsed}
    {contextWindowSize}
    {defaultModelId}
    onSendAndCopy={handleSendAndCopy}
    onContextAutoShow={handleContextAutoShow}
    onCloseContext={closeContext}
    onToggleContext={toggleContext}
  />
  <TabSidebar {store} open={sidebarOpen} onClose={() => (sidebarOpen = false)} />
</div>

<ChatPalette
  open={chatPaletteOpen}
  onClose={() => (chatPaletteOpen = false)}
  onNewChat={handleChatPaletteNewChat}
  onOpenConversation={handleChatPaletteOpenConversation}
/>

<style>
  :global(html),
  :global(body) {
    background: var(--surface-base);
  }

  .dialog-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--surface-base);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--font-size-base);
    overflow: hidden;
    position: relative;
  }
</style>
