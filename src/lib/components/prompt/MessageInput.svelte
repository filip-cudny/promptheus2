<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import { getClipboardImage } from "$lib/utils/paste";
  import type { createConversationStore } from "$lib/stores/conversation.svelte";
  import type { ConversationImage } from "$lib/types/conversation";

  let {
    store,
    onSendAndCopy,
  }: {
    store: ReturnType<typeof createConversationStore>;
    onSendAndCopy: () => void;
  } = $props();

  let textarea: HTMLTextAreaElement | undefined = $state();
  let localText = $state("");
  let localImages = $state<ConversationImage[]>([]);

  $effect(() => {
    store.updateInputText(localText);
  });

  $effect(() => {
    store.updateInputImages(localImages);
  });

  $effect(() => {
    if (store.inputText === "" && localText !== "") {
      localText = "";
    }
  });

  $effect(() => {
    if (store.inputImages.length === 0 && localImages.length > 0) {
      localImages = [];
    }
  });

  onMount(() => {
    textarea?.focus();
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      getCurrentWindow().close();
      return;
    }

    if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      if (store.isRegenerateMode) {
        const path = store.tree.current_path;
        if (path.length > 0) {
          store.regenerate(path[path.length - 1]);
        }
      } else if (store.canSend) {
        store.sendMessage();
      }
      return;
    }

    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      if (store.canSend) {
        onSendAndCopy();
      }
      return;
    }
  }

  async function handlePaste() {
    const image = await getClipboardImage();
    if (image) {
      localImages = [...localImages, image];
    }
  }
</script>

<div class="message-input">
  <ImageChipBar bind:images={localImages} readonly={false} />
  <textarea
    bind:this={textarea}
    class="input-textarea"
    bind:value={localText}
    placeholder="Type a message… (Enter to send, Shift+Enter for newline, Ctrl+Enter to send & copy, Esc to close)"
    onkeydown={handleKeydown}
    onpaste={handlePaste}
    disabled={store.isExecuting}
  ></textarea>
</div>

<style>
  .message-input {
    flex-shrink: 0;
    padding: 0 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .input-textarea {
    width: 100%;
    min-height: 60px;
    max-height: 200px;
    resize: vertical;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    color: #e0e0e0;
    font: inherit;
    font-size: 13px;
    padding: 10px;
    box-sizing: border-box;
  }

  .input-textarea:focus {
    outline: none;
    border-color: rgba(100, 160, 255, 0.5);
  }

  .input-textarea:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
