<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ContextSection from "./ContextSection.svelte";
  import AttachMenu from "./AttachMenu.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import { SendHorizonal, RefreshCw, Square, CopyCheck } from "lucide-svelte";
  import { getClipboardImage } from "$lib/utils/paste";
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

  async function handlePaste() {
    const image = await getClipboardImage();
    if (image) {
      localImages = [...localImages, image];
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
      <ActionIconButton
        icon={CopyCheck}
        onclick={onSendAndCopy}
        disabled={!store.canSend || store.isExecuting}
        title="Send & Copy (Ctrl+Enter)"
      />

      {#if store.isExecuting}
        <ActionIconButton
          icon={Square}
          onclick={() => store.stopExecution()}
          title="Stop"
        />
      {:else if store.isRegenerateMode}
        <ActionIconButton
          icon={RefreshCw}
          onclick={handleSendShow}
          title="Regenerate"
        />
      {:else}
        <ActionIconButton
          icon={SendHorizonal}
          onclick={handleSendShow}
          disabled={!store.canSend}
          title="Send (Enter)"
        />
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
    gap: 2px;
  }
</style>
