<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import { getImageFromPasteEvent } from "$lib/utils/paste";
  import { autoResize, resizeTextarea } from "$lib/utils/autoResize";
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

  $effect(() => {
    localText;
    if (textarea) requestAnimationFrame(() => resizeTextarea(textarea!));
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      const abortNodeId = store.abortRegenerateNodeId;
      if (abortNodeId) {
        store.editAndRegenerate(abortNodeId, localText.trim());
      } else if (store.isRegenerateMode) {
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

  async function handlePaste(e: ClipboardEvent) {
    const image = await getImageFromPasteEvent(e);
    if (image) {
      localImages = [...localImages, image];
    }
  }
</script>

<div class="message-input">
  <ImageChipBar bind:images={localImages} readonly={false} onopen={(image) => invoke("open_image_preview", { data: image.data, mediaType: image.media_type })} />
  <textarea
    bind:this={textarea}
    class="input-textarea"
    bind:value={localText}
    use:autoResize={"40vh"}
    rows="1"
    placeholder="Type a message…"
    onkeydown={handleKeydown}
    onpaste={handlePaste}
    disabled={store.isExecuting}
  ></textarea>
</div>

<style>
  .message-input {
    flex-shrink: 0;
    padding: var(--space-0) var(--space-8);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .input-textarea {
    width: 100%;
    min-height: 0;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    color: var(--text-primary);
    font: inherit;
    font-size: var(--font-size-base);
    padding: var(--space-5);
    box-sizing: border-box;
  }

  .input-textarea:focus {
    outline: none;
    border-color: var(--accent-border);
  }

  .input-textarea:disabled {
    opacity: var(--opacity-disabled);
    cursor: not-allowed;
  }
</style>
