<script lang="ts">
  import ImageChipBar from "$lib/components/shared/ui/ImageChipBar.svelte";
  import { openImagePreview } from "$lib/services/windowPreviews";
  import { getImageFromPasteEvent } from "$lib/utils/paste";
  import { autoResize, resizeTextarea } from "$lib/utils/autoResize";
  import type { ConversationImage } from "$lib/types/conversation";

  let {
    text = $bindable(),
    images = $bindable(),
    disabled = false,
    placeholder = "Enter context text…",
    variant = "default",
    hideChipRow = false,
  }: {
    text: string;
    images: ConversationImage[];
    disabled?: boolean;
    placeholder?: string;
    variant?: "default" | "flat";
    hideChipRow?: boolean;
  } = $props();

  let textarea: HTMLTextAreaElement | undefined = $state();

  $effect(() => {
    text;
    if (textarea) requestAnimationFrame(() => resizeTextarea(textarea!));
  });

  async function handlePaste(e: ClipboardEvent) {
    if (disabled) return;
    const image = await getImageFromPasteEvent(e);
    if (image) {
      const number = Math.max(0, ...images.map((existing) => existing.number ?? 0)) + 1;
      images = [...images, { ...image, number }];
    }
  }
</script>

<div class="context-editor" class:flat={variant === "flat"}>
  {#if !hideChipRow && images.length > 0}
    <div class="chip-row">
      <ImageChipBar bind:images readonly={disabled} onopen={(image) => openImagePreview(image.data, image.media_type)} />
    </div>
  {/if}
  <textarea
    bind:this={textarea}
    class="context-textarea"
    bind:value={text}
    use:autoResize={"none"}
    rows="1"
    {placeholder}
    {disabled}
    onpaste={handlePaste}
  ></textarea>
</div>

<style>
  .context-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    padding: var(--space-4) var(--space-4) var(--space-0);
  }

  .context-editor.flat {
    flex: 1;
    background: transparent;
    border: none;
    border-radius: 0;
    overflow: hidden;
  }

  .chip-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
  }

  .context-editor:focus-within {
    border-color: var(--accent-border);
  }

  .context-editor.flat:focus-within {
    border-color: transparent;
  }

  .context-textarea {
    width: 100%;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font: inherit;
    font-size: var(--font-size-base);
    padding: var(--space-2) var(--space-0) var(--space-4);
    box-sizing: border-box;
    overflow: hidden;
  }

  .context-editor.flat > .context-textarea {
    flex: 1;
    min-height: 100px;
    max-height: none;
    height: auto;
    resize: none;
    overflow-y: auto;
  }

  .context-textarea:focus {
    outline: none;
  }

  .context-textarea:disabled {
    cursor: not-allowed;
  }
</style>
