<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ContextEditor from "$lib/components/ui/ContextEditor.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import { Save, Check, X } from "lucide-svelte";
  import {
    getContextItems,
    clearContext,
    setContext,
    setContextImage,
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
        await setContextImage(img.data, img.media_type);
      }
      await getCurrentWindow().close();
    } finally {
      saving = false;
    }
  }

  async function cancel() {
    await getCurrentWindow().close();
  }
</script>

<div class="editor-shell">
  <div class="editor-content">
    <ContextEditor bind:text bind:images />
  </div>
  <div class="editor-footer">
    <ActionIconButton
      icon={Save}
      confirmIcon={Check}
      onclick={save}
      title="Save and close"
      disabled={saving}
    />
    <ActionIconButton
      icon={X}
      onclick={cancel}
      title="Cancel"
    />
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
    gap: 8px;
  }

  .editor-content {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .editor-content :global(.context-editor) {
    flex: 1;
  }

  .editor-content :global(.context-textarea) {
    flex: 1;
    max-height: none;
    min-height: 100px;
  }

  .editor-footer {
    display: flex;
    justify-content: flex-end;
    gap: 4px;
    flex-shrink: 0;
  }
</style>
