<script lang="ts">
  import type { ConversationImage } from "$lib/types/conversation";

  let {
    images = $bindable(),
    readonly = false,
  }: {
    images: ConversationImage[];
    readonly: boolean;
  } = $props();

  function removeImage(index: number) {
    images = images.filter((_, i) => i !== index);
  }

  function thumbnailSrc(image: ConversationImage): string {
    return `data:${image.media_type};base64,${image.data}`;
  }
</script>

{#if images.length > 0}
  <div class="image-chip-bar">
    {#each images as image, idx}
      <div class="image-chip">
        <img src={thumbnailSrc(image)} alt="Attached image {idx + 1}" class="chip-thumbnail" />
        {#if !readonly}
          <button class="chip-delete" onclick={() => removeImage(idx)}>✕</button>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .image-chip-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 4px 0;
  }

  .image-chip {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: rgba(255, 255, 255, 0.05);
  }

  .chip-thumbnail {
    width: 40px;
    height: 40px;
    object-fit: cover;
  }

  .chip-delete {
    position: absolute;
    top: -2px;
    right: -2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: none;
    background: rgba(200, 60, 60, 0.9);
    color: #fff;
    font-size: 10px;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .chip-delete:hover {
    background: rgba(220, 50, 50, 1);
  }
</style>
