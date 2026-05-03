<script lang="ts">
  import { onMount } from "svelte";
  import ContextEditorView from "$lib/components/features/context-editor/ContextEditorView.svelte";
  import { useContextLoadSave } from "$lib/components/features/context-editor/drivers/useContextLoadSave.svelte";
  import type { ConversationImage } from "$lib/types/conversation";

  const ctx = useContextLoadSave();

  let text = $state("");
  let images = $state<ConversationImage[]>([]);
  let editMode = $state(true);

  onMount(async () => {
    const loaded = await ctx.load();
    text = loaded.text;
    images = loaded.images;
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      if (!ctx.saving) ctx.save(text, images);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="editor-shell">
  <ContextEditorView
    bind:text
    bind:images
    bind:editMode
    saving={ctx.saving}
    errorMessage={ctx.errorMessage}
    onSave={() => ctx.save(text, images)}
  />
</div>

<style>
  .editor-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--surface-base);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--font-size-base);
    padding: var(--space-6);
    box-sizing: border-box;
  }
</style>
