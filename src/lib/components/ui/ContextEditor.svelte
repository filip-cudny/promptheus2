<script lang="ts">
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import type { ConversationImage } from "$lib/types/conversation";

  let {
    text = $bindable(),
    images = $bindable(),
    readonly = false,
    disabled = false,
    placeholder = "Enter context text\u2026",
  }: {
    text: string;
    images: ConversationImage[];
    readonly?: boolean;
    disabled?: boolean;
    placeholder?: string;
  } = $props();
</script>

<div class="context-editor">
  <ImageChipBar bind:images readonly={readonly || disabled} />
  <textarea
    class="context-textarea"
    bind:value={text}
    {placeholder}
    disabled={disabled || readonly}
  ></textarea>
</div>

<style>
  .context-editor {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .context-textarea {
    width: 100%;
    min-height: 50px;
    max-height: 120px;
    resize: vertical;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    color: #e0e0e0;
    font: inherit;
    font-size: 13px;
    padding: 8px;
    box-sizing: border-box;
  }

  .context-textarea:focus {
    outline: none;
    border-color: rgba(100, 160, 255, 0.5);
  }

  .context-textarea:disabled {
    cursor: not-allowed;
  }
</style>
