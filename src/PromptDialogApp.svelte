<script lang="ts">
  import { onDestroy } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { createConversationStore } from "$lib/stores/conversation.svelte";
  import ContextSection from "$lib/components/prompt/ContextSection.svelte";
  import ConversationArea from "$lib/components/prompt/ConversationArea.svelte";
  import MessageInput from "$lib/components/prompt/MessageInput.svelte";
  import ButtonBar from "$lib/components/prompt/ButtonBar.svelte";

  const params = new URLSearchParams(window.location.search);
  const promptId = params.get("promptId") ?? "";
  const promptName = params.get("promptName") ?? "Prompt";

  const store = createConversationStore(promptId, promptName);

  async function handleSendAndCopy() {
    const currentWindow = getCurrentWindow();
    const { success, result } = await store.sendMessage();
    if (success && result) {
      await navigator.clipboard.writeText(result);
      await currentWindow.close();
    }
  }

  onDestroy(() => {
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
