<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { createConversationStore } from "$lib/stores/conversation.svelte";
  import ContextSection from "$lib/components/prompt/ContextSection.svelte";
  import ConversationArea from "$lib/components/prompt/ConversationArea.svelte";
  import MessageInput from "$lib/components/prompt/MessageInput.svelte";
  import ButtonBar from "$lib/components/prompt/ButtonBar.svelte";

  const params = new URLSearchParams(window.location.search);
  const promptId = params.get("promptId") ?? "";
  const promptName = params.get("promptName") ?? "Prompt";
  const historyEntryId = params.get("historyEntryId");

  const store = createConversationStore(promptId, promptName);

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
  });

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
  <ContextSection {store} />
  <ConversationArea {store} />
  <MessageInput {store} onSendAndCopy={handleSendAndCopy} />
  <ButtonBar {store} onSendAndCopy={handleSendAndCopy} />
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
