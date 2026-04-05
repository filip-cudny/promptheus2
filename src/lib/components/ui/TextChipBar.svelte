<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import AttachmentChip from "./AttachmentChip.svelte";

  const PREVIEW_MAX_CHARS = 200;

  type Variant = "default" | "small";

  let {
    textAttachments = $bindable(),
    readonly = false,
    variant = "default" as Variant,
    onremove,
  }: {
    textAttachments: string[];
    readonly?: boolean;
    variant?: Variant;
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

  function truncate(text: string): string {
    if (text.length <= PREVIEW_MAX_CHARS) return text;
    return text.slice(0, PREVIEW_MAX_CHARS) + "…";
  }
</script>

{#each textAttachments as text, idx}
  <AttachmentChip label="Text #{idx + 1}" {readonly} {variant} onclick={() => openPreview(text, idx)} onremove={() => removeAttachment(idx)}>
    {#snippet content()}
      <span class="chip-preview">{truncate(text)}</span>
    {/snippet}
  </AttachmentChip>
{/each}

<style>
  .chip-preview {
    font-size: 9px;
    line-height: 1.3;
    color: rgba(255, 255, 255, 0.55);
    overflow: hidden;
    word-break: break-word;
    padding: 4px 4px 0;
    height: 100%;
  }
</style>
