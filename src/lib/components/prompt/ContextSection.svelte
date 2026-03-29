<script lang="ts">
  import { onMount } from "svelte";
  import CollapsibleSection from "$lib/components/ui/CollapsibleSection.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import { getContextItems, setContext, setContextImage, clearContext } from "$lib/services/context";
  import { Save, Check, X } from "lucide-svelte";
  import type { createConversationStore } from "$lib/stores/conversation.svelte";
  import type { ConversationImage } from "$lib/types/conversation";

  let {
    store,
    contextDisabled = false,
    initialCollapsed = false,
    onHasContent,
    onClose,
  }: {
    store: ReturnType<typeof createConversationStore>;
    contextDisabled?: boolean;
    initialCollapsed?: boolean;
    onHasContent?: () => void;
    onClose?: () => void;
  } = $props();

  let collapsed = $state(initialCollapsed);
  let saving = $state(false);
  let confirming = $state(false);

  let localText = $state("");
  let localImages = $state<ConversationImage[]>([]);

  let hasContent = $derived(localText.trim().length > 0 || localImages.length > 0);
  let contextHeaderClass = $derived(collapsed && hasContent ? "context-has-content" : "");

  $effect(() => {
    store.updateContextText(localText);
  });

  $effect(() => {
    store.updateContextImages(localImages);
  });

  onMount(async () => {
    try {
      const items = await getContextItems();
      for (const item of items) {
        if (item.item_type === "text") {
          localText = item.content;
        } else if (item.item_type === "image") {
          localImages = [
            ...localImages,
            { data: item.data, media_type: item.media_type },
          ];
        }
      }
      if (localText.trim().length > 0 || localImages.length > 0) {
        onHasContent?.();
      }
    } catch {
      // non-fatal
    }
  });

  async function saveContext() {
    saving = true;
    try {
      await clearContext();
      if (localText.trim()) {
        await setContext(localText);
      }
      for (const img of localImages) {
        await setContextImage(img.data, img.media_type);
      }
    } finally {
      saving = false;
    }
  }

  function requestClose() {
    if (hasContent) {
      confirming = true;
    } else {
      onClose?.();
    }
  }

  function confirmClose() {
    confirming = false;
    localText = "";
    localImages = [];
    onClose?.();
  }

  function cancelClose() {
    confirming = false;
  }
</script>

<div class="context-inline" class:disabled={contextDisabled}>
  {#if confirming}
    <div class="confirm-bar">
      <span class="confirm-text">Clear context for this conversation?</span>
      <div class="confirm-actions">
        <button class="confirm-btn confirm-yes" onclick={confirmClose}>Clear</button>
        <button class="confirm-btn confirm-no" onclick={cancelClose}>Cancel</button>
      </div>
    </div>
  {:else}
    <CollapsibleSection title="Context" bind:collapsed headerClass={contextHeaderClass}>
      {#snippet actions()}
        <ActionIconButton
          icon={Save}
          confirmIcon={Check}
          onclick={saveContext}
          title="Save context globally"
          disabled={contextDisabled || saving}
        />
        <ActionIconButton
          icon={X}
          onclick={requestClose}
          title="Close context"
        />
      {/snippet}
      <div class="context-body">
        <ImageChipBar bind:images={localImages} readonly={contextDisabled} />
        <textarea
          class="context-textarea"
          bind:value={localText}
          placeholder={contextDisabled ? "This prompt doesn't use {{context}}" : "Enter context text…"}
          disabled={contextDisabled}
        ></textarea>
      </div>
    </CollapsibleSection>
  {/if}
</div>

<style>
  .context-inline {
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .context-inline.disabled {
    opacity: 0.5;
  }

  .context-inline :global(.collapsible-section) {
    border: none;
    border-radius: 0;
  }

  .context-inline :global(.collapsible-header) {
    background: transparent;
    padding: 6px 10px;
    font-size: 12px;
  }

  .context-inline :global(.collapsible-header:hover) {
    background: rgba(255, 255, 255, 0.04);
  }

  .context-inline :global(.collapsible-header.context-has-content) {
    background: rgba(100, 160, 255, 0.08);
  }

  .context-inline :global(.collapsible-header.context-has-content .collapsible-title) {
    color: #7dd3f0;
    font-weight: 700;
  }

  .context-inline :global(.collapsible-content) {
    padding: 6px 10px 8px;
  }

  .confirm-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 10px;
    background: rgba(220, 60, 60, 0.08);
  }

  .confirm-text {
    font-size: 12px;
    color: #e0e0e0;
  }

  .confirm-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .confirm-btn {
    padding: 4px 10px;
    border-radius: 4px;
    border: none;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }

  .confirm-yes {
    background: rgba(220, 60, 60, 0.8);
    color: #fff;
  }

  .confirm-yes:hover {
    background: rgba(220, 60, 60, 1);
  }

  .confirm-no {
    background: rgba(255, 255, 255, 0.1);
    color: #e0e0e0;
  }

  .confirm-no:hover {
    background: rgba(255, 255, 255, 0.18);
  }

  .context-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .context-textarea {
    width: 100%;
    min-height: 50px;
    max-height: 120px;
    resize: vertical;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    color: #e0e0e0;
    font: inherit;
    font-size: 13px;
    padding: 8px;
    box-sizing: border-box;
  }

  .context-textarea:focus {
    outline: none;
    border-color: rgba(100, 160, 255, 0.5);
  }

  .context-textarea:disabled {
    cursor: not-allowed;
  }
</style>
