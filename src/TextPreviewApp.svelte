<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { emitTo } from "@tauri-apps/api/event";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import MarkdownRenderer from "$lib/components/ui/MarkdownRenderer.svelte";
  import { resizeTextarea } from "$lib/utils/autoResize";
  import EditorToolbar from "$lib/components/ui/EditorToolbar.svelte";

  const win = getCurrentWebviewWindow();

  let text = $state("");
  let index = $state(0);
  let sourceWindow = $state("");
  let editMode = $state(false);
  let textarea: HTMLTextAreaElement | undefined = $state();

  let lineCount = $derived(text ? text.split("\n").length : 0);
  let originalText = $state("");
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
    invoke("hide_dialog_window", { label: "text-preview" });
  }

  function handleInput(e: Event) {
    text = (e.target as HTMLTextAreaElement).value;
  }

  onMount(() => {
    const unlistenLoad = win.listen("load-text", () => {
      loadText();
    });

    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") hide();
    };
    window.addEventListener("keydown", handleKey);

    return () => {
      unlistenLoad.then((fn) => fn());
      window.removeEventListener("keydown", handleKey);
    };
  });
</script>

<div class="text-preview">
  <EditorToolbar {lineCount} bind:editMode saveDisabled={!isDirty} onsave={hide} />

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
        <MarkdownRenderer content={text} isStreaming={false} />
      </div>
    {/if}
  </div>
</div>

<style>
  .text-preview {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #1e1e1e;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  .markdown-view {
    font-size: 14px;
    line-height: 1.6;
    color: #e0e0e0;
  }

  .content:has(.edit-textarea) {
    display: flex;
    flex-direction: column;
  }

  .edit-textarea {
    width: 100%;
    flex: 1;
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    color: #e0e0e0;
    font-family: "Fira Code", "Cascadia Code", monospace;
    font-size: 13px;
    line-height: 1.5;
    padding: 8px;
    resize: none;
    box-sizing: border-box;
  }

  .edit-textarea:focus {
    outline: none;
    border-color: rgba(74, 158, 187, 0.4);
  }
</style>
