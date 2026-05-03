<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { emitTo } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { save } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import MarkdownRenderer from "$lib/components/shared/ui/MarkdownRenderer.svelte";
  import { resizeTextarea } from "$lib/utils/autoResize";
  import { countTokensDebounced } from "$lib/services/tokenCounter";
  import EditorToolbar from "$lib/components/shared/ui/EditorToolbar.svelte";

  const win = getCurrentWebviewWindow();

  let text = $state("");
  let index = $state(0);
  let sourceWindow = $state("");
  let editMode = $state(false);
  let textarea: HTMLTextAreaElement | undefined = $state();

  let lineCount = $derived(text ? text.split("\n").length : 0);
  let tokenCount = $state(0);
  let originalText = $state("");

  $effect(() => {
    countTokensDebounced(text, "openai", (count) => { tokenCount = count; });
  });
  let isDirty = $derived(text !== originalText);

  $effect(() => {
    text;
    if (editMode && textarea) {
      requestAnimationFrame(() => resizeTextarea(textarea!));
    }
  });

  async function loadText() {
    const payload = await invoke<{
      text: string;
      index: number;
      source_window: string;
    } | null>("get_pending_text");
    if (!payload) return;

    text = payload.text;
    originalText = payload.text;
    index = payload.index;
    sourceWindow = payload.source_window;
    editMode = false;
  }

  function hide() {
    emitTo(sourceWindow, "text-attachment-updated", { text, index });
    invoke("save_text_preview_geometry");
    invoke("hide_dialog_window", { label: win.label });
  }

  async function copyText() {
    await navigator.clipboard.writeText(text);
  }

  function handleInput(e: Event) {
    text = (e.target as HTMLTextAreaElement).value;
  }

  async function handleSaveSvg(svg: string) {
    const path = await save({
      defaultPath: "mermaid-diagram.svg",
      filters: [{ name: "SVG", extensions: ["svg"] }],
    });
    if (path) await invoke("write_text_file", { path, content: svg });
  }

  onMount(() => {
    loadText();

    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") hide();
    };
    window.addEventListener("keydown", handleKey);

    return () => {
      window.removeEventListener("keydown", handleKey);
    };
  });
</script>

<div class="text-preview">
  <EditorToolbar {lineCount} {tokenCount} bind:editMode saveDisabled={!isDirty} onsave={hide} oncopy={copyText} />

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
