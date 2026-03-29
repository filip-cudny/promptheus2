<script lang="ts">
  import type { ConversationImage } from "$lib/types/conversation";

  let {
    images = $bindable(),
    readonly = false,
  }: {
    images: ConversationImage[];
    readonly: boolean;
  } = $props();

  let previewIndex = $state<number | null>(null);

  function removeImage(index: number) {
    images = images.filter((_, i) => i !== index);
  }

  function thumbnailSrc(image: ConversationImage): string {
    return `data:${image.media_type};base64,${image.data}`;
  }

  function togglePreview(idx: number) {
    previewIndex = previewIndex === idx ? null : idx;
  }
</script>

{#if images.length > 0}
  <div class="image-chip-bar">
    {#each images as image, idx}
      <div class="image-chip">
        <button class="chip-thumbnail-btn" onclick={() => togglePreview(idx)}>
          <img src={thumbnailSrc(image)} alt="Attached image {idx + 1}" class="chip-thumbnail" />
        </button>
        {#if !readonly}
          <button class="chip-delete" onclick={() => removeImage(idx)}>✕</button>
        {/if}
        {#if previewIndex === idx}
          <button class="preview-popup" onclick={() => (previewIndex = null)}>
            <img
              src={thumbnailSrc(image)}
              alt="Preview"
              class="preview-image"
            />
          </button>
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
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: rgba(255, 255, 255, 0.05);
  }

  .chip-thumbnail-btn {
    padding: 0;
    border: none;
    background: none;
    cursor: pointer;
    display: flex;
  }

  .chip-thumbnail {
    width: 40px;
    height: 40px;
    object-fit: cover;
    border-radius: 5px;
  }

  .chip-delete {
    position: absolute;
    top: -2px;
    right: -2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: none;
    background: rgba(255, 255, 255, 0.2);
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
    background: rgba(255, 255, 255, 0.35);
  }

  .preview-popup {
    position: absolute;
    bottom: calc(100% + 6px);
    left: 0;
    z-index: 100;
    width: 400px;
    height: 400px;
    padding: 0;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 8px;
    background: rgba(30, 30, 30, 0.95);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.6);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .preview-image {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 7px;
  }
</style>
