<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ContextEditor from "$lib/components/ui/ContextEditor.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import { Save, Check } from "lucide-svelte";
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
  let confirmed = $state(false);

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
    if (e.key === "Escape") {
      e.preventDefault();
      getCurrentWindow().close();
      return;
    }
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      if (!saving) {
        save();
        confirmed = true;
        setTimeout(() => (confirmed = false), 1200);
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="editor-shell">
  <div class="editor-content">
    <ContextEditor bind:text bind:images />
    <div class="button-bar">
      {#if errorMessage}
        <span class="save-error">{errorMessage}</span>
      {/if}
      <ActionIconButton
        icon={Save}
        confirmIcon={Check}
        onclick={save}
        title="Save (Ctrl+Enter)"
        disabled={saving}
        bind:confirmed
      />
    </div>
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
    padding: 8px 8px 0;
    border: none;
    border-radius: 0;
    background: transparent;
  }

  .editor-content :global(.context-editor:focus-within) {
    border-color: transparent;
  }

  .editor-content :global(.context-textarea) {
    flex: 1;
    max-height: none;
    min-height: 100px;
    resize: none;
  }

  .button-bar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex-shrink: 0;
    padding: 6px 8px;
    gap: 8px;
  }

  .save-error {
    flex: 1;
    font-size: 11px;
    color: #e55;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
