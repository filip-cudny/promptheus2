<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ContextSection from "./ContextSection.svelte";
  import AttachMenu from "./AttachMenu.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import type { createConversationStore } from "$lib/stores/conversation.svelte";
  import type { ConversationImage } from "$lib/types/conversation";

  let {
    store,
    contextVisible,
    contextDisabled,
    contextInitialCollapsed = false,
    onSendAndCopy,
    onContextAutoShow,
    onCloseContext,
    onToggleContext,
  }: {
    store: ReturnType<typeof createConversationStore>;
    contextVisible: boolean;
    contextDisabled: boolean;
    contextInitialCollapsed?: boolean;
    onSendAndCopy: () => void;
    onContextAutoShow: () => void;
    onCloseContext: () => void;
    onToggleContext: () => void;
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

  const primaryLabel = $derived.by(() => {
    if (store.isExecuting) return "Stop";
    if (store.isRegenerateMode) return "Regenerate";
    return "Send";
  });

  function handleSendShow() {
    if (store.isRegenerateMode) {
      const path = store.tree.current_path;
      if (path.length > 0) {
        store.regenerate(path[path.length - 1]);
      }
    } else {
      store.sendMessage();
    }
  }

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

  function handlePaste(e: ClipboardEvent) {
    const items = e.clipboardData?.items;
    if (!items) return;

    for (const item of items) {
      if (item.type.startsWith("image/")) {
        e.preventDefault();
        const blob = item.getAsFile();
        if (!blob) return;

        const reader = new FileReader();
        reader.onload = () => {
          const result = reader.result as string;
          const base64 = result.split(",")[1];
          const mediaType = item.type;
          localImages = [
            ...localImages,
            { data: base64, media_type: mediaType },
          ];
        };
        reader.readAsDataURL(blob);
        return;
      }
    }
  }
</script>

<div class="input-area">
  {#if contextVisible}
    <ContextSection {store} {contextDisabled} initialCollapsed={contextInitialCollapsed} onHasContent={onContextAutoShow} onClose={onCloseContext} />
  {/if}

  <div class="input-field">
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

  <div class="button-bar">
    <div class="bar-left">
      <AttachMenu onSelectContext={onToggleContext} {contextDisabled} />
    </div>

    <div class="bar-right">
      <button
        class="btn btn-secondary"
        onclick={onSendAndCopy}
        disabled={!store.canSend || store.isExecuting}
        title="Ctrl+Enter"
      >
        Send & Copy
      </button>

      {#if store.isExecuting}
        <button class="btn btn-danger" onclick={() => store.stopExecution()}>
          Stop
        </button>
      {:else}
        <button
          class="btn btn-primary"
          onclick={handleSendShow}
          disabled={!store.canSend && !store.isRegenerateMode}
          title="Enter"
        >
          {primaryLabel}
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .input-area {
    flex-shrink: 0;
    position: relative;
    z-index: 10;
    margin: -8px 16px 8px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 8px;
    background: rgba(30, 30, 30, 0.75);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  :global([data-platform="linux"]) .input-area {
    background: rgba(30, 30, 30, 0.95);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }

  .input-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 8px 0;
  }

  .input-textarea {
    width: 100%;
    min-height: 60px;
    max-height: 200px;
    resize: vertical;
    background: transparent;
    border: none;
    color: #e0e0e0;
    font: inherit;
    font-size: 13px;
    padding: 4px 2px;
    box-sizing: border-box;
  }

  .input-textarea:focus {
    outline: none;
  }

  .input-textarea:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .button-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 8px;
  }

  .bar-left {
    flex-shrink: 0;
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .bar-right {
    flex-shrink: 0;
    display: flex;
    gap: 6px;
  }

  .btn {
    padding: 6px 14px;
    border-radius: 4px;
    border: none;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-primary {
    background: rgba(100, 160, 255, 0.8);
    color: #fff;
  }

  .btn-primary:hover:not(:disabled) {
    background: rgba(100, 160, 255, 1);
  }

  .btn-secondary {
    background: rgba(255, 255, 255, 0.1);
    color: #e0e0e0;
  }

  .btn-secondary:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.18);
  }

  .btn-danger {
    background: rgba(220, 60, 60, 0.8);
    color: #fff;
  }

  .btn-danger:hover {
    background: rgba(220, 60, 60, 1);
  }
</style>
