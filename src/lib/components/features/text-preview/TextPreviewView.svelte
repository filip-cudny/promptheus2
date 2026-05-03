<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { save as saveDialog } from "@tauri-apps/plugin-dialog";
  import EditorToolbar from "$lib/components/shared/ui/EditorToolbar.svelte";
  import MarkdownRenderer from "$lib/components/shared/ui/MarkdownRenderer.svelte";
  import { resizeTextarea } from "$lib/utils/autoResize";
  import { countTokensDebounced } from "$lib/services/tokenCounter";

  let {
    text = $bindable(),
    editMode = $bindable(false),
    isDirty,
    onSave,
    onCopy,
  }: {
    text: string;
    editMode?: boolean;
    isDirty: boolean;
    onSave: () => void;
    onCopy: () => void;
  } = $props();

  let textarea: HTMLTextAreaElement | undefined = $state();
  let lineCount = $derived(text ? text.split("\n").length : 0);
  let tokenCount = $state(0);

  $effect(() => {
    countTokensDebounced(text, "openai", (count) => {
      tokenCount = count;
    });
  });

  $effect(() => {
    text;
    if (editMode && textarea) {
      requestAnimationFrame(() => resizeTextarea(textarea!));
    }
  });

  function handleInput(e: Event) {
    text = (e.target as HTMLTextAreaElement).value;
  }

  async function handleSaveSvg(svg: string) {
    const path = await saveDialog({
      defaultPath: "mermaid-diagram.svg",
      filters: [{ name: "SVG", extensions: ["svg"] }],
    });
    if (path) await invoke("write_text_file", { path, content: svg });
  }
</script>

<div class="text-preview">
  <EditorToolbar {lineCount} {tokenCount} bind:editMode saveDisabled={!isDirty} onsave={onSave} oncopy={onCopy} />

  <div class="content">
    {#if editMode}
      <textarea
        bind:this={textarea}
        value={text}
        oninput={handleInput}
        class="edit-textarea"
      ></textarea>
    {:else}
      <div class="markdown-view">
        <MarkdownRenderer content={text} isStreaming={false} onopen={openUrl} onsavesvg={handleSaveSvg} />
      </div>
    {/if}
  </div>
</div>

<style>
  .text-preview {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--surface-base);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-6);
  }

  .markdown-view {
    font-size: var(--font-size-lg);
    line-height: var(--line-height-relaxed);
    color: var(--text-primary);
  }

  .content:has(.edit-textarea) {
    display: flex;
    flex-direction: column;
  }

  .edit-textarea {
    width: 100%;
    flex: 1;
    background: transparent;
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--font-size-base);
    line-height: var(--line-height-normal);
    padding: var(--space-4);
    resize: none;
    box-sizing: border-box;
  }

  .edit-textarea:focus {
    outline: none;
    border-color: rgba(74, 158, 187, 0.4);
  }
</style>
