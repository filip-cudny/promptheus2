<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { createConversationStore } from "$lib/stores/conversation.svelte";
  import { hasContext } from "$lib/services/context";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { PanelLeft, SquarePen } from "lucide-svelte";
  import { getSkillsStore } from "$lib/stores/skills.svelte";
  import ConversationArea from "$lib/components/prompt/ConversationArea.svelte";
  import InputArea from "$lib/components/prompt/InputArea.svelte";
  import TabSidebar from "$lib/components/prompt/TabSidebar.svelte";

  const skillsStore = getSkillsStore();

  const params = new URLSearchParams(window.location.search);
  const promptId = params.get("promptId") ?? "";
  const promptName = params.get("promptName") ?? "Prompt";
  const historyEntryId = params.get("historyEntryId");
  const lastInteractionOnly = params.get("lastInteractionOnly") === "true";
  const initialInput = params.get("initialInput");
  const autoSendInput = params.get("autoSendInput") === "true";

  const store = createConversationStore(promptId, promptName);

  let sidebarOpen = $state(false);
  let contextVisible = $state(false);
  let contextDisabled = $state(false);
  let contextInitialCollapsed = $state(false);

  let unlistenRestore: UnlistenFn | undefined;
  let unlistenContextChanged: UnlistenFn | undefined;
  let unlistenVoiceInput: UnlistenFn | undefined;

  async function autoShowContextIfNeeded() {
    if (contextDisabled || contextVisible) return;
    if (await hasContext()) {
      contextInitialCollapsed = true;
      contextVisible = true;
    }
  }

  function handleVoiceInput(text: string, autoSend: boolean) {
    const currentTab = store.tabs.find(t => t.tab_id === store.activeTabId);
    if (currentTab && currentTab.tree.current_path.length > 0) {
      store.addTab();
    }
    const inputText = promptId ? `/${promptId} ${text}` : text;
    store.updateInputText(inputText);
    if (autoSend) {
      store.sendMessage();
    }
  }

  onMount(async () => {
    skillsStore.init();

    if (historyEntryId) {
      await store.restoreFromHistory(historyEntryId, lastInteractionOnly);
    } else if (initialInput) {
      handleVoiceInput(initialInput, autoSendInput);
    } else if (promptId) {
      store.updateInputText(`/${promptId} `);
    }

    unlistenRestore = await listen<{ entry_id: string; last_interaction_only?: boolean }>(
      "restore-history",
      (event) => {
        store.restoreFromHistory(event.payload.entry_id, event.payload.last_interaction_only);
      },
    );

    unlistenVoiceInput = await listen<{ text: string; auto_send: boolean }>("voice-input", (event) => {
      handleVoiceInput(event.payload.text, event.payload.auto_send);
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
    unlistenRestore?.();
    unlistenContextChanged?.();
    unlistenVoiceInput?.();
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
      {#if store.tabs.length > 1}
        <span class="tab-badge">{store.tabs.length}</span>
      {/if}
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
  <InputArea {store} {contextVisible} {contextDisabled} {contextInitialCollapsed} onSendAndCopy={handleSendAndCopy} onContextAutoShow={handleContextAutoShow} onCloseContext={closeContext} onToggleContext={toggleContext} />
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

  .tab-badge {
    position: absolute;
    top: -5px;
    right: -2px;
    min-width: 15px;
    height: 15px;
    border-radius: 8px;
    background: rgba(100, 160, 255, 0.85);
    color: #fff;
    font-size: 9px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 4px;
    box-sizing: border-box;
  }
</style>
