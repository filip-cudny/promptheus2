<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { X, FileText } from "lucide-svelte";

  let {
    textAttachments = $bindable(),
    readonly = false,
    onremove,
  }: {
    textAttachments: string[];
    readonly?: boolean;
    onremove?: (index: number) => void;
  } = $props();

  function removeAttachment(index: number) {
    if (onremove) {
      onremove(index);
    } else {
      textAttachments = textAttachments.filter((_, i) => i !== index);
    }
  }

  function openPreview(text: string, index: number) {
    const sourceWindow = getCurrentWebviewWindow().label;
    invoke("open_text_preview", { text, index, sourceWindow });
  }
</script>

{#if textAttachments.length > 0}
  <div class="text-chip-bar">
    {#each textAttachments as text, idx}
      <div class="text-chip">
        <button class="chip-thumbnail-btn" onclick={() => openPreview(text, idx)}>
          <div class="chip-icon">
            <FileText size={18} strokeWidth={1.5} />
            <span class="chip-label">Text #{idx + 1}</span>
          </div>
        </button>
        {#if !readonly}
          <button class="chip-delete" onclick={() => removeAttachment(idx)}>
            <X size={11} strokeWidth={2.5} />
          </button>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .text-chip-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 2px 0;
  }

  .text-chip {
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

  .chip-icon {
    width: 40px;
    height: 40px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1px;
    color: rgba(255, 255, 255, 0.5);
    border-radius: 5px;
  }

  .chip-label {
    font-size: 8px;
    font-weight: 600;
    letter-spacing: 0.5px;
    color: rgba(255, 255, 255, 0.4);
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
