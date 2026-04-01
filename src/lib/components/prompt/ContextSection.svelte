<script lang="ts">
  import { onMount } from "svelte";
  import CollapsibleSection from "$lib/components/ui/CollapsibleSection.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import ContextEditor from "$lib/components/ui/ContextEditor.svelte";
  import { getContextItems, appendContext, appendContextImage, clearContext } from "$lib/services/context";
  import { Save, Check, X, Trash2 } from "lucide-svelte";
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
  let confirming = $state<"close" | "clear" | false>(false);

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
      for (const img of localImages) {
        await appendContextImage(img.data, img.media_type);
      }
      if (localText.trim()) {
        await appendContext(localText);
      }
    } finally {
      saving = false;
    }
  }

  function requestClose() {
    if (hasContent) {
      confirming = "close";
    } else {
      onClose?.();
    }
  }

  function requestClear() {
    confirming = "clear";
  }

  function confirmAction() {
    if (confirming === "close") {
      localText = "";
      localImages = [];
      confirming = false;
      onClose?.();
    } else if (confirming === "clear") {
      localText = "";
      localImages = [];
      confirming = false;
    }
  }

  function cancelConfirm() {
    confirming = false;
  }
</script>

<div class="context-inline" class:disabled={contextDisabled}>
  {#if confirming}
    <div class="confirm-bar">
      <span class="confirm-text">
        {confirming === "close" ? "Clear context and close?" : "Clear context for this conversation?"}
      </span>
      <div class="confirm-actions">
        <button class="confirm-btn confirm-yes" onclick={confirmAction}>Clear</button>
        <button class="confirm-btn confirm-no" onclick={cancelConfirm}>Cancel</button>
      </div>
    </div>
  {:else}
    <CollapsibleSection title="Context" bind:collapsed headerClass={contextHeaderClass}>
      {#snippet actions()}
        <ActionIconButton
          icon={Trash2}
          onclick={requestClear}
          title="Clear context"
          disabled={contextDisabled || !hasContent}
        />
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
      <ContextEditor
        bind:text={localText}
        bind:images={localImages}
        disabled={contextDisabled}
        placeholder={contextDisabled ? "This prompt doesn't use {{context}}" : "Enter context text\u2026"}
      />
    </CollapsibleSection>
  {/if}
</div>

<style>
  .context-inline {
    border-bottom: 1px solid rgba(255, 255, 255, 0.15);
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
    max-height: 35vh;
    overflow-y: auto;
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

</style>
