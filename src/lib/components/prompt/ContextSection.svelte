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
    border-bottom: 1px solid var(--border-strong);
  }

  .context-inline.disabled {
    opacity: var(--opacity-disabled);
  }

  .context-inline :global(.collapsible-section) {
    border: none;
    border-radius: 0;
  }

  .context-inline :global(.collapsible-header) {
    background: transparent;
    padding: var(--space-3) var(--space-5);
    font-size: var(--font-size-md);
  }

  .context-inline :global(.collapsible-header:hover) {
    background: var(--surface-overlay-faint);
  }

  .context-inline :global(.collapsible-header.context-has-content) {
    background: var(--accent-bg-soft);
  }

  .context-inline :global(.collapsible-header.context-has-content .collapsible-title) {
    color: var(--info);
    font-weight: 700;
  }

  .context-inline :global(.collapsible-content) {
    padding: var(--space-3) var(--space-5) var(--space-4);
    max-height: 35vh;
    overflow-y: auto;
  }

  .confirm-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-5);
    background: var(--danger-bg-soft);
  }

  .confirm-text {
    font-size: var(--font-size-md);
    color: var(--text-primary);
  }

  .confirm-actions {
    display: flex;
    gap: var(--space-3);
    flex-shrink: 0;
  }

  .confirm-btn {
    padding: var(--space-2) var(--space-5);
    border-radius: var(--radius-md);
    border: none;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
  }

  .confirm-yes {
    background: var(--danger);
    color: var(--text-primary);
  }

  .confirm-yes:hover {
    background: var(--danger);
  }

  .confirm-no {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }

  .confirm-no:hover {
    background: rgba(255, 255, 255, 0.18);
  }

</style>
