<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { X } from "lucide-svelte";
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

  function openPreview(image: ConversationImage) {
    invoke("open_image_preview", {
      data: image.data,
      mediaType: image.media_type,
    });
  }
</script>

{#if images.length > 0}
  <div class="image-chip-bar">
    {#each images as image, idx}
      <div class="image-chip">
        <button class="chip-thumbnail-btn" onclick={() => openPreview(image)}>
          <img src={thumbnailSrc(image)} alt="Attached image {idx + 1}" class="chip-thumbnail" />
        </button>
        {#if !readonly}
          <button class="chip-delete" onclick={() => removeImage(idx)}><X size={11} strokeWidth={2.5} /></button>
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
    top: -3px;
    right: -3px;
    width: 19px;
    height: 19px;
    border-radius: 50%;
    border: 1px solid #555;
    background: #333;
    color: #fff;
    cursor: pointer;
    display: grid;
    place-items: center;
    padding: 0;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
  }

  .chip-delete:hover {
    background: #444;
  }
</style>
