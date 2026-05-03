<script lang="ts">
  import AttachmentChip from "./AttachmentChip.svelte";
  import type { ConversationImage } from "$lib/types/conversation";

  type Variant = "default" | "small";

  let {
    images = $bindable(),
    readonly = false,
    variant = "default" as Variant,
    onremove,
    onopen,
  }: {
    images: ConversationImage[];
    readonly?: boolean;
    variant?: Variant;
    onremove?: (index: number) => void;
    onopen?: (image: ConversationImage) => void;
  } = $props();

  function removeImage(index: number) {
    if (onremove) {
      onremove(index);
    } else {
      images = images.filter((_, i) => i !== index);
    }
  }

  function thumbnailSrc(image: ConversationImage): string {
    return `data:${image.media_type};base64,${image.data}`;
  }
</script>

{#each images as image, idx}
  <AttachmentChip label="Image #{idx + 1}" {readonly} {variant} onclick={() => onopen?.(image)} onremove={() => removeImage(idx)}>
    {#snippet content()}
      <img src={thumbnailSrc(image)} alt="Attached image {idx + 1}" class="chip-thumbnail" />
    {/snippet}
  </AttachmentChip>
{/each}

<style>
  .chip-thumbnail {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
