<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ContextSection from "./ContextSection.svelte";
  import AttachMenu from "./AttachMenu.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import TextChipBar from "$lib/components/ui/TextChipBar.svelte";
  import SkillEditable from "$lib/components/ui/SkillEditable.svelte";
  import { SendHorizonal, RefreshCw, Square, CopyCheck } from "lucide-svelte";
  import { getImageFromPasteEvent, extractTextAttachment } from "$lib/utils/paste";
  import { TEXT_ATTACHMENT_CHAR_THRESHOLD } from "$lib/constants/ui";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
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

  let localText = $state("");
  let localImages = $state<ConversationImage[]>([]);
  let localTextAttachments = $state<string[]>([]);
  let shiftHeld = $state(false);
  let skillEditable: ReturnType<typeof SkillEditable> | undefined = $state();

  $effect(() => {
    store.updateInputText(localText);
  });

  $effect(() => {
    store.updateInputImages(localImages);
  });

  $effect(() => {
    store.updateInputTextAttachments(localTextAttachments);
  });

  $effect(() => {
    const storeText = store.inputText;
    if (storeText === "" && localText !== "") {
      localText = "";
      const el = skillEditable?.getElement();
      if (el) {
        el.innerHTML = "";
      }
    } else if (storeText !== "" && localText === "" && skillEditable) {
      localText = storeText;
      skillEditable.setTextAndHighlight(storeText);
      requestAnimationFrame(() => {
        skillEditable?.focus();
        skillEditable?.restoreCursor(storeText.length);
      });
    }
  });

  $effect(() => {
    if (store.inputImages.length === 0 && localImages.length > 0) {
      localImages = [];
    }
  });

  $effect(() => {
    if (store.inputTextAttachments.length === 0 && localTextAttachments.length > 0) {
      localTextAttachments = [];
    }
  });

  let unlistenTextUpdate: (() => void) | null = null;

  function onKeyDown(e: KeyboardEvent) { shiftHeld = e.shiftKey; }
  function onKeyUp(e: KeyboardEvent) { shiftHeld = e.shiftKey; }

  onMount(async () => {
    skillEditable?.focus();
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);

    const win = getCurrentWebviewWindow();
    unlistenTextUpdate = await win.listen<{ text: string; index: number }>(
      "text-attachment-updated",
      (event) => {
        const { text, index } = event.payload;
        if (index >= 0 && index < localTextAttachments.length) {
          localTextAttachments = localTextAttachments.map((t, i) =>
            i === index ? text : t,
          );
        }
      },
    );
  });

  onDestroy(() => {
    unlistenTextUpdate?.();
    window.removeEventListener("keydown", onKeyDown);
    window.removeEventListener("keyup", onKeyUp);
  });

  function sendOrRegenerate() {
    if (store.isRegenerateMode) {
      const path = store.tree.current_path;
      if (path.length > 0) {
        store.regenerate(path[path.length - 1]);
      }
    } else if (store.canSend) {
      store.sendMessage();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      sendOrRegenerate();
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
    const textAttachment = !shiftHeld ? extractTextAttachment(e, TEXT_ATTACHMENT_CHAR_THRESHOLD) : null;
    if (textAttachment) {
      requestAnimationFrame(() => {
        localTextAttachments = [...localTextAttachments, textAttachment];
      });
      return;
    }

    const plainText = e.clipboardData?.getData("text/plain") ?? "";
    e.preventDefault();

    const image = await getImageFromPasteEvent(e);
    if (image) {
      localImages = [...localImages, image];
      return;
    }
    if (plainText) {
      document.execCommand("insertText", false, plainText);
    }
  }
</script>

<div class="input-area">
  {#if contextVisible}
    <ContextSection {store} {contextDisabled} initialCollapsed={contextInitialCollapsed} onHasContent={onContextAutoShow} onClose={onCloseContext} />
  {/if}

  <div class="input-field">
    {#if localTextAttachments.length > 0 || localImages.length > 0}
      <div class="attachment-row">
        <TextChipBar bind:textAttachments={localTextAttachments} readonly={false} />
        <ImageChipBar bind:images={localImages} readonly={false} />
      </div>
    {/if}
    <SkillEditable
      bind:this={skillEditable}
      bind:text={localText}
      placeholder="Type a message… (use /skill-name for skills)"
      editableClass="input-editable"
      onkeydown={handleKeydown}
      onpaste={handlePaste}
    />
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
          onclick={sendOrRegenerate}
          title="Regenerate"
        />
      {:else}
        <ActionIconButton
          icon={SendHorizonal}
          onclick={sendOrRegenerate}
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

  .attachment-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 6px 0 2px;
    max-height: 15vh;
    overflow-y: auto;
  }

  .input-field :global(.input-editable) {
    font-size: 13px;
    max-height: 35vh;
    overflow-y: auto;
    padding: 4px 2px;
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
