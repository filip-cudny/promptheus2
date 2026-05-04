<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { save as saveDialog } from "@tauri-apps/plugin-dialog";
  import { writeTextFile } from "@tauri-apps/plugin-fs";
  import ContextEditor from "$lib/components/shared/widgets/ContextEditor.svelte";
  import ImageChipBar from "$lib/components/shared/ui/ImageChipBar.svelte";
  import EditorToolbar from "$lib/components/shared/ui/EditorToolbar.svelte";
  import MarkdownRenderer from "$lib/components/shared/ui/MarkdownRenderer.svelte";
  import { countTokensDebounced, estimateImageTokens } from "$lib/services/tokenCounter";
  import type { ConversationImage } from "$lib/types/conversation";

  let {
    text = $bindable(),
    images = $bindable(),
    editMode = $bindable(true),
    saving,
    errorMessage,
    onSave,
  }: {
    text: string;
    images: ConversationImage[];
    editMode?: boolean;
    saving: boolean;
    errorMessage: string;
    onSave: () => void;
  } = $props();

  let lineCount = $derived(text ? text.split("\n").length : 0);
  let textTokens = $state(0);
  let tokenCount = $derived(textTokens + images.length * estimateImageTokens("openai"));

  $effect(() => {
    countTokensDebounced(text, "openai", (count) => {
      textTokens = count;
    });
  });

  async function handleSaveSvg(svg: string) {
    const path = await saveDialog({
      defaultPath: "mermaid-diagram.svg",
      filters: [{ name: "SVG", extensions: ["svg"] }],
    });
    if (path) await writeTextFile(path, svg);
  }
</script>

<div class="editor-content">
  <EditorToolbar {lineCount} {tokenCount} bind:editMode saveDisabled={saving} onsave={onSave} />
  {#if errorMessage}
    <span class="save-error">{errorMessage}</span>
  {/if}
  {#if images.length > 0}
    <div class="image-row">
      <ImageChipBar bind:images readonly={!editMode} onopen={(image) => invoke("open_image_preview", { data: image.data, mediaType: image.media_type })} />
    </div>
  {/if}
  {#if editMode}
    <ContextEditor bind:text bind:images variant="flat" hideChipRow />
  {:else}
    <div class="markdown-view">
      <MarkdownRenderer content={text} isStreaming={false} onopen={openUrl} onsavesvg={handleSaveSvg} />
    </div>
  {/if}
</div>

<style>
  .editor-content {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xl);
    background: rgba(30, 30, 30, 0.75);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  .image-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    max-height: 35vh;
    overflow-y: auto;
    padding: var(--space-4) var(--space-4) var(--space-0);
  }

  .image-row :global(.chip-wrapper) {
    width: 80px;
    height: 80px;
  }

  .markdown-view {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-6);
    font-size: var(--font-size-lg);
    line-height: var(--line-height-relaxed);
    color: var(--text-primary);
  }

  .save-error {
    font-size: var(--font-size-sm);
    color: var(--danger);
    padding: var(--space-2) var(--space-4) var(--space-0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
