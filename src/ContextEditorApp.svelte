<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ContextEditor from "$lib/components/ui/ContextEditor.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import { Save, Check } from "lucide-svelte";
  import {
    getContextItems,
    clearContext,
    setContext,
    appendContextImage,
  } from "$lib/services/context";
  import type { ConversationImage } from "$lib/types/conversation";

  let text = $state("");
  let images = $state<ConversationImage[]>([]);
  let saving = $state(false);

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
    try {
      await clearContext();
      if (text.trim()) {
        await setContext(text);
      }
      for (const img of images) {
        await appendContextImage(img.data, img.media_type);
      }
      await getCurrentWindow().close();
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
      if (!saving) save();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="editor-shell">
  <div class="editor-content">
    <ContextEditor bind:text bind:images />
    <div class="button-bar">
      <ActionIconButton
        icon={Save}
        confirmIcon={Check}
        onclick={save}
        title="Save and close (Ctrl+Enter)"
        disabled={saving}
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
  }

  .editor-content :global(.context-textarea) {
    flex: 1;
    max-height: none;
    min-height: 100px;
    background: transparent;
    border: none;
  }

  .editor-content :global(.context-textarea:focus) {
    border-color: transparent;
  }

  .button-bar {
    display: flex;
    justify-content: flex-end;
    flex-shrink: 0;
    padding: 6px 8px;
  }
</style>
