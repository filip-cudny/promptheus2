<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { createConversationStore } from "$lib/stores/conversation.svelte";
  import { getSettings } from "$lib/services/settings";
  import ConversationArea from "$lib/components/prompt/ConversationArea.svelte";
  import InputArea from "$lib/components/prompt/InputArea.svelte";

  const params = new URLSearchParams(window.location.search);
  const promptId = params.get("promptId") ?? "";
  const promptName = params.get("promptName") ?? "Prompt";
  const historyEntryId = params.get("historyEntryId");

  const store = createConversationStore(promptId, promptName);

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
  <ConversationArea {store} />
  <InputArea {store} {contextVisible} {contextDisabled} {contextInitialCollapsed} onSendAndCopy={handleSendAndCopy} onContextAutoShow={handleContextAutoShow} onCloseContext={closeContext} onToggleContext={toggleContext} />
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
  }
</style>
