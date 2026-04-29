<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { createConversationStore } from "$lib/stores/conversation.svelte";
  import { hasContext } from "$lib/services/context";
  import { getSettings } from "$lib/services/settings";
  import { getContextWindowSize } from "$lib/utils/contextWindow";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { PanelLeft, SquarePen } from "lucide-svelte";
  import { getSkillsStore } from "$lib/stores/skills.svelte";
  import ChatPalette from "$lib/components/prompt/ChatPalette.svelte";
  import ConversationArea from "$lib/components/prompt/ConversationArea.svelte";
  import InputArea from "$lib/components/prompt/InputArea.svelte";
  import TabSidebar from "$lib/components/prompt/TabSidebar.svelte";
  import { openPalette, reloadActiveInHost } from "$lib/services/shellToolbar";

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

  import type { ModelConfig } from "$lib/types";

  let sidebarOpen = $state(false);
  let contextVisible = $state(false);
  let contextDisabled = $state(false);
  let contextInitialCollapsed = $state(false);
  let models = $state<ModelConfig[]>([]);
  let defaultModelId = $state<string | null>(null);

  let chatPaletteOpen = $state(false);

  let contextWindowSize = $derived.by(() => {
    const activeModelId = store.modelId ?? defaultModelId;
    const activeModel = models.find((m) => m.id === activeModelId);
    if (!activeModel) return 0;
    return getContextWindowSize(activeModel.model, activeModel.context_window_size);
  });

  let unlistenRestore: UnlistenFn | undefined;
  let unlistenContextChanged: UnlistenFn | undefined;
  let unlistenVoiceInput: UnlistenFn | undefined;
  let unlistenOpenForSkill: UnlistenFn | undefined;
  let unlistenNewConversation: UnlistenFn | undefined;

  async function handleGlobalKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "r") {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (chatPaletteOpen) {
        chatPaletteOpen = false;
      }
      await reloadActiveProvider();
      return;
    }

    if (chatPaletteOpen) {
      return;
    }

    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "p") {
      e.preventDefault();
      e.stopPropagation();
      try {
        await openPalette(HOST_LABEL);
      } catch (err) {
        console.error("open_palette failed", err);
      }
      return;
    }

    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "k") {
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
    const currentTab = store.tabs.find(t => t.tab_id === store.activeTabId);
    if (currentTab && currentTab.tree.current_path.length > 0) {
      store.addTab();
    }
    const inputText = skillId ? `/${skillId} ${text}` : text;
    store.updateInputText(inputText);
    if (autoSend) {
      store.sendMessage();
    }
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

  onMount(async () => {
    window.addEventListener("keydown", handleGlobalKeydown, true);
    skillsStore.init();
    await store.initFromSettings();
    loadModelInfo();

    const reconnected = await store.tryReconnect();

    const initParams = await invoke<DialogInitParams | null>("get_dialog_init_params");
    if (!reconnected && initParams) {
      await applyInitParams(initParams);
    }

    unlistenRestore = await listen<{ entry_id: string; last_interaction_only?: boolean }>(
      "restore-history",
      (event) => {
        store.restoreFromHistory(event.payload.entry_id, event.payload.last_interaction_only);
      },
      { target: SELF_TARGET },
    );

    unlistenVoiceInput = await listen<{ text: string; auto_send: boolean }>(
      "voice-input",
      (event) => {
        handleVoiceInput("", event.payload.text, event.payload.auto_send);
      },
      { target: SELF_TARGET },
    );

    unlistenOpenForSkill = await listen<{ skill_id: string; skill_name: string; skill_model: string | null }>(
      "open-for-skill",
      (event) => {
        store.openForSkill(event.payload.skill_id, event.payload.skill_name, event.payload.skill_model);
      },
      { target: SELF_TARGET },
    );

    unlistenNewConversation = await listen(
      "new-conversation",
      () => {
        store.addTab();
      },
      { target: SELF_TARGET },
    );

    await autoShowContextIfNeeded();

    unlistenContextChanged = await listen("context-changed", () => {
      autoShowContextIfNeeded();
    });
  });

  function handleContextAutoShow() {
    if (!contextDisabled) {
      contextInitialCollapsed = !contextVisible;
      contextVisible = true;
    }
  }

  function toggleContext() {
    if (contextVisible) {
      closeContext();
    } else {
      contextVisible = true;
    }
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
    window.removeEventListener("keydown", handleGlobalKeydown, true);
    unlistenRestore?.();
    unlistenContextChanged?.();
    unlistenVoiceInput?.();
    unlistenOpenForSkill?.();
    unlistenNewConversation?.();
    store.destroy();
  });
</script>

<div class="dialog-shell">
  <div class="top-buttons" class:sidebar-open={sidebarOpen}>
    <button
      class="top-btn sidebar-toggle"
      class:hidden={sidebarOpen}
      onclick={() => sidebarOpen = !sidebarOpen}
      title="Toggle conversations"
    >
      <PanelLeft size={ICON_SIZE.md} />
    </button>
    <button
      class="top-btn"
      onclick={() => store.addTab()}
      title="New conversation"
    >
      <SquarePen size={ICON_SIZE.md} />
    </button>
  </div>
  <ConversationArea {store} />
  <InputArea {store} {models} {contextVisible} {contextDisabled} {contextInitialCollapsed} {contextWindowSize} {defaultModelId} onSendAndCopy={handleSendAndCopy} onContextAutoShow={handleContextAutoShow} onCloseContext={closeContext} onToggleContext={toggleContext} />
  <TabSidebar {store} open={sidebarOpen} onClose={() => sidebarOpen = false} />
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
    background: #1e1e1e;
  }

  .dialog-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #1e1e1e;
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 13px;
    overflow: hidden;
    position: relative;
  }

  .top-buttons {
    position: absolute;
    top: 6px;
    left: 6px;
    z-index: 201;
    display: flex;
    gap: 4px;
    transition: transform 0.2s ease;
  }

  .top-buttons.sidebar-open {
    transform: translateX(240px);
  }

  .sidebar-toggle {
    width: 28px;
    overflow: visible;
    transition: width 0.2s ease, opacity 0.2s ease;
  }

  .sidebar-toggle.hidden {
    width: 0;
    opacity: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .top-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: none;
    background: rgba(255, 255, 255, 0.03);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    color: rgba(255, 255, 255, 0.35);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    position: relative;
  }

  :global([data-platform="linux"]) .top-btn {
    background: rgba(255, 255, 255, 0.06);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }

  .top-btn:hover {
    color: rgba(255, 255, 255, 0.8);
    background: rgba(255, 255, 255, 0.1);
  }
</style>
