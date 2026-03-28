<script lang="ts">
  import { onMount } from "svelte";
  import CollapsibleSection from "$lib/components/ui/CollapsibleSection.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import { getContextItems, setContext, setContextImage, clearContext } from "$lib/services/context";
  import { getSettings } from "$lib/services/settings";
  import type { createConversationStore } from "$lib/stores/conversation.svelte";
  import type { ConversationImage } from "$lib/types/conversation";

  let {
    store,
  }: {
    store: ReturnType<typeof createConversationStore>;
  } = $props();

  let collapsed = $state(false);
  let disabled = $state(false);
  let saving = $state(false);

  let localText = $state("");
  let localImages = $state<ConversationImage[]>([]);

  $effect(() => {
    store.updateContextText(localText);
  });

  $effect(() => {
    store.updateContextImages(localImages);
  });

  const params = new URLSearchParams(window.location.search);
  const promptId = params.get("promptId") ?? "";

  onMount(async () => {
    try {
      const settings = await getSettings();
      const prompt = settings.prompts.find((p) => p.id === promptId);
      if (prompt) {
        const usesContext = prompt.messages.some((m) =>
          m.content.includes("{{context}}"),
        );
        disabled = !usesContext;
      }
    } catch {
      // leave context enabled if settings unavailable
    }

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
</script>

<div class="context-section" class:disabled>
  <CollapsibleSection title="Context" bind:collapsed>
    {#snippet actions()}
      <button class="save-btn" onclick={saveContext} disabled={disabled || saving}>
        {saving ? "Saving…" : "Save"}
      </button>
    {/snippet}
    <div class="context-body">
      <ImageChipBar bind:images={localImages} readonly={disabled} />
      <textarea
        class="context-textarea"
        bind:value={localText}
        placeholder={disabled ? "This prompt doesn't use {{context}}" : "Enter context text…"}
        {disabled}
      ></textarea>
    </div>
  </CollapsibleSection>
</div>

<style>
  .context-section {
    flex-shrink: 0;
    padding: 8px 12px 0;
  }

  .context-section.disabled {
    opacity: 0.5;
  }

  .context-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .context-textarea {
    width: 100%;
    min-height: 60px;
    max-height: 150px;
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

  .save-btn {
    padding: 2px 10px;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: rgba(255, 255, 255, 0.08);
    color: #e0e0e0;
    font-size: 12px;
    cursor: pointer;
  }

  .save-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.15);
  }

  .save-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
