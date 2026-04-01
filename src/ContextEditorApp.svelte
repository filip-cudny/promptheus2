<script lang="ts">
  import { onMount } from "svelte";
  import ContextEditor from "$lib/components/ui/ContextEditor.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import EditorToolbar from "$lib/components/ui/EditorToolbar.svelte";
  import MarkdownRenderer from "$lib/components/ui/MarkdownRenderer.svelte";
  import {
    getContextItems,
    clearContext,
    appendContext,
    appendContextImage,
  } from "$lib/services/context";
  import type { ConversationImage } from "$lib/types/conversation";

  let text = $state("");
  let images = $state<ConversationImage[]>([]);
  let saving = $state(false);
  let errorMessage = $state("");
  let editMode = $state(true);
  let lineCount = $derived(text ? text.split("\n").length : 0);

  onMount(async () => {
    const items = await getContextItems();
    for (const item of items) {
      if (item.item_type === "text") {
        text += (text ? "\n" : "") + item.content;
      } else if (item.item_type === "image") {
        images = [...images, { data: item.data, media_type: item.media_type }];
      }
    }
  });

  async function save() {
    saving = true;
    errorMessage = "";
    const textSnapshot = text;
    const imageSnapshot = [...images];
    try {
      await clearContext();
      for (const img of imageSnapshot) {
        await appendContextImage(img.data, img.media_type);
      }
      if (textSnapshot.trim()) {
        await appendContext(textSnapshot);
      }
    } catch (e) {
      errorMessage = e instanceof Error ? e.message : String(e);
      console.error("Failed to save context:", e);
    } finally {
      saving = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      if (!saving) {
        save();
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="editor-shell">
  <div class="editor-content">
    <EditorToolbar {lineCount} bind:editMode saveDisabled={saving} onsave={save} />
    {#if errorMessage}
      <span class="save-error">{errorMessage}</span>
    {/if}
    {#if images.length > 0}
      <div class="image-row">
        <ImageChipBar bind:images readonly={!editMode} />
      </div>
    {/if}
    {#if editMode}
      <ContextEditor bind:text bind:images />
    {:else}
      <div class="markdown-view">
        <MarkdownRenderer content={text} isStreaming={false} />
      </div>
    {/if}
  </div>
</div>

<style>
  .editor-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #1e1e1e;
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 13px;
    padding: 12px;
    box-sizing: border-box;
  }

  .editor-content {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 8px;
    background: rgba(30, 30, 30, 0.75);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  .editor-content :global(.context-editor) {
    flex: 1;
    padding: 8px;
    border: none;
    border-radius: 0;
    background: transparent;
    overflow: hidden;
  }

  .editor-content :global(.context-editor:focus-within) {
    border-color: transparent;
  }

  .editor-content :global(.context-editor > .chip-row) {
    display: none;
  }

  .image-row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    max-height: 35vh;
    overflow-y: auto;
    padding: 8px 8px 0;
  }

  .image-row :global(.chip-wrapper) {
    width: 80px;
    height: 80px;
  }

  .editor-content :global(.context-textarea) {
    flex: 1;
    min-height: 100px;
    max-height: none !important;
    height: auto !important;
    resize: none;
    overflow-y: auto;
  }

  .markdown-view {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    font-size: 14px;
    line-height: 1.6;
    color: #e0e0e0;
  }

  .save-error {
    font-size: 11px;
    color: #e55;
    padding: 4px 8px 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
