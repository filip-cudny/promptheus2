<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { createConversationStore } from "$lib/stores/conversation.svelte";
  import { getSettings } from "$lib/services/settings";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { PanelLeft } from "lucide-svelte";
  import ConversationArea from "$lib/components/prompt/ConversationArea.svelte";
  import InputArea from "$lib/components/prompt/InputArea.svelte";
  import TabSidebar from "$lib/components/prompt/TabSidebar.svelte";

  const params = new URLSearchParams(window.location.search);
  const promptId = params.get("promptId") ?? "";
  const promptName = params.get("promptName") ?? "Prompt";
  const historyEntryId = params.get("historyEntryId");

  const store = createConversationStore(promptId, promptName);

  let sidebarOpen = $state(false);
  let contextVisible = $state(false);
  let contextDisabled = $state(false);
  let contextInitialCollapsed = $state(false);

  let unlistenRestore: UnlistenFn | undefined;

  onMount(async () => {
    if (historyEntryId) {
      await store.restoreFromHistory(historyEntryId);
    }

    unlistenRestore = await listen<{ entry_id: string }>(
      "restore-history",
      (event) => {
        store.restoreFromHistory(event.payload.entry_id);
      },
    );

    try {
      const settings = await getSettings();
      const prompt = settings.prompts.find((p) => p.id === promptId);
      if (prompt) {
        const usesContext = prompt.messages.some((m) =>
          m.content.includes("{{context}}"),
        );
        contextDisabled = !usesContext;
      }
    } catch {
      // leave context enabled if settings unavailable
    }
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
    store.destroy();
  });
</script>

<div class="dialog-shell">
  <button
    class="sidebar-toggle"
    onclick={() => sidebarOpen = !sidebarOpen}
    title="Toggle conversations"
  >
    <PanelLeft size={ICON_SIZE.md} />
    {#if store.tabs.length > 1}
      <span class="tab-badge">{store.tabs.length}</span>
    {/if}
  </button>
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

  .sidebar-toggle {
    position: absolute;
    top: 6px;
    left: 6px;
    z-index: 50;
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
  }

  .sidebar-toggle:hover {
    color: rgba(255, 255, 255, 0.8);
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.25);
  }

  .tab-badge {
    position: absolute;
    top: -4px;
    right: -6px;
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
    padding: 0 3px;
  }
</style>
