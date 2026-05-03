<script lang="ts">
  import ImageChipBar from "$lib/components/shared/ui/ImageChipBar.svelte";
  import TextChipBar from "$lib/components/shared/ui/TextChipBar.svelte";
  import type { ConversationImage } from "$lib/types/conversation";

  let {
    textAttachments,
    images,
    readonly = false,
    variant = "default",
    onRemoveText,
    onRemoveImage,
    onOpenText,
    onOpenImage,
  }: {
    textAttachments: string[];
    images: ConversationImage[];
    readonly?: boolean;
    variant?: "small" | "default";
    onRemoveText?: (idx: number) => void;
    onRemoveImage?: (idx: number) => void;
    onOpenText: (text: string, index: number) => void;
    onOpenImage: (image: ConversationImage) => void;
  } = $props();

  const hasContent = $derived(textAttachments.length > 0 || images.length > 0);
</script>

{#if hasContent}
  <div class="attachment-row" class:small={variant === "small"}>
    <TextChipBar
      {textAttachments}
      {readonly}
      {variant}
      onremove={onRemoveText}
      onopen={onOpenText}
    />
    <ImageChipBar
      {images}
      {readonly}
      {variant}
      onremove={onRemoveImage}
      onopen={onOpenImage}
    />
  </div>
{/if}

<style>
  .attachment-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    padding: var(--space-1) var(--space-0);
  }

  .attachment-row.small {
    flex-wrap: nowrap;
    padding: var(--space-3) var(--space-0) var(--space-1);
    overflow-x: auto;
  }
</style>
