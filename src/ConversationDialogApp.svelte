<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { createConversationStore } from "$lib/stores/conversation.svelte";
  import { hasContext } from "$lib/services/context";
  import { getSettings } from "$lib/services/settings";
  import { getContextWindowSize } from "$lib/utils/contextWindow";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { PanelLeft, SquarePen } from "lucide-svelte";
  import { getSkillsStore } from "$lib/stores/skills.svelte";
  import {
    getAiProviders,
    openAiWebviewNewWindow,
    swapAiWebview,
    type AiProvider,
  } from "$lib/services/aiWebview";
  import { openConversationDialogNewWindow } from "$lib/services/conversationDialog";
  import "$lib/providerSwitcher.js";
  import ConversationArea from "$lib/components/prompt/ConversationArea.svelte";
  import InputArea from "$lib/components/prompt/InputArea.svelte";
  import TabSidebar from "$lib/components/prompt/TabSidebar.svelte";

  const PROMPTHEUS_PROVIDER_ID = "promptheus";

  type SwitcherProvider = { id: string; name: string };
  type SwitcherHandle = {
    element: HTMLElement;
    setActive: (id: string) => void;
    destroy: () => void;
  };
  type SwitcherFactory = (opts: {
    providers: SwitcherProvider[];
    activeId: string;
    newWindowTitle?: string;
    onSelect: (id: string) => void;
    onOpenNewWindow: (id: string) => void;
  }) => SwitcherHandle;

  interface DialogInitParams {
    skill_id: string;
    skill_name: string;
    history_entry_id: string | null;
    last_interaction_only: boolean;
    initial_input: string | null;
    auto_send_input: boolean;
    new_chat: boolean;
  }

  const skillsStore = getSkillsStore();

  const store = createConversationStore("", "");

  import type { ModelConfig } from "$lib/types";

  let sidebarOpen = $state(false);
  let contextVisible = $state(false);
  let contextDisabled = $state(false);
  let contextInitialCollapsed = $state(false);
  let models = $state<ModelConfig[]>([]);
  let defaultModelId = $state<string | null>(null);
  let aiProviders = $state<AiProvider[]>([]);
  let switcherMountEl: HTMLDivElement | undefined = $state();
  let switcherHandle: SwitcherHandle | null = null;

  async function handleSwitcherSelect(providerId: string) {
    if (providerId === PROMPTHEUS_PROVIDER_ID) return;
    try {
      await swapAiWebview(providerId, getCurrentWindow().label);
    } catch (e) {
      console.error("failed to swap ai webview", e);
    }
  }

  async function handleSwitcherOpenNewWindow(providerId: string) {
    try {
      if (providerId === PROMPTHEUS_PROVIDER_ID) {
        await openConversationDialogNewWindow();
      } else {
        await openAiWebviewNewWindow(providerId);
      }
    } catch (err) {
      console.error("failed to open provider in new window", err);
    }
  }

  function mountSwitcher() {
    if (!switcherMountEl || switcherHandle) return;
    const factory = (globalThis as unknown as {
      __promptheusSwitcher?: { create: SwitcherFactory };
    }).__promptheusSwitcher?.create;
    if (!factory) return;
    const providers: SwitcherProvider[] = [
      { id: PROMPTHEUS_PROVIDER_ID, name: "Promptheus" },
      ...aiProviders.map((p) => ({ id: p.id, name: p.name })),
    ];
    switcherHandle = factory({
      providers,
      activeId: PROMPTHEUS_PROVIDER_ID,
      newWindowTitle: "Otwórz w nowym oknie",
      onSelect: handleSwitcherSelect,
      onOpenNewWindow: handleSwitcherOpenNewWindow,
    });
    switcherMountEl.appendChild(switcherHandle.element);
  }

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

  function handleGlobalKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && store.isExecuting) {
      e.preventDefault();
      store.abortExecution();
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
      store.openForSkill(p.skill_id, p.skill_name);
    }
  }

  async function loadModelInfo() {
    try {
      const settings = await getSettings();
      models = settings.models;
      defaultModelId = settings.surfaces.chat.generation.model_id ?? null;
    } catch {}
  }

  async function loadAiProviders() {
    try {
      aiProviders = await getAiProviders();
      mountSwitcher();
    } catch (e) {
      console.error("failed to load ai providers", e);
    }
  }

  onMount(async () => {
    window.addEventListener("keydown", handleGlobalKeydown);
    skillsStore.init();
    await store.initFromSettings();
    loadModelInfo();
    loadAiProviders();

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
    );

    unlistenVoiceInput = await listen<{ text: string; auto_send: boolean }>("voice-input", (event) => {
      handleVoiceInput("", event.payload.text, event.payload.auto_send);
    });

    unlistenOpenForSkill = await listen<{ skill_id: string; skill_name: string }>("open-for-skill", (event) => {
      store.openForSkill(event.payload.skill_id, event.payload.skill_name);
    });

    unlistenNewConversation = await listen("new-conversation", () => {
      store.addTab();
    });

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
    window.removeEventListener("keydown", handleGlobalKeydown);
    unlistenRestore?.();
    unlistenContextChanged?.();
    unlistenVoiceInput?.();
    unlistenOpenForSkill?.();
    unlistenNewConversation?.();
    switcherHandle?.destroy();
    switcherHandle = null;
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
    <div class="switcher-mount" bind:this={switcherMountEl}></div>
  </div>
  <ConversationArea {store} />
  <InputArea {store} {models} {contextVisible} {contextDisabled} {contextInitialCollapsed} {contextWindowSize} {defaultModelId} onSendAndCopy={handleSendAndCopy} onContextAutoShow={handleContextAutoShow} onCloseContext={closeContext} onToggleContext={toggleContext} />
  <TabSidebar {store} open={sidebarOpen} onClose={() => sidebarOpen = false} />
</div>

<style>
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

  .switcher-mount {
    display: inline-flex;
    align-items: center;
  }
</style>
