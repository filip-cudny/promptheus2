<script lang="ts">
  import { onMount } from "svelte";
  import { Save, Check } from "lucide-svelte";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import {
    getPrompt,
    savePrompt,
    type PromptDoc,
    type PromptKind,
  } from "$lib/services/prompts";
  import EnvPlaceholdersPanel from "./EnvPlaceholdersPanel.svelte";

  const AUTOSAVE_DEBOUNCE_MS = 800;

  let {
    kind,
    description,
  }: {
    kind: PromptKind;
    description?: string;
  } = $props();

  let doc = $state<PromptDoc | null>(null);
  let content = $state("");
  let savedContent = $state("");
  let textarea = $state<HTMLTextAreaElement | undefined>(undefined);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let lastSavedAt = $state<number | null>(null);
  let autosaveTimer: ReturnType<typeof setTimeout> | null = null;

  let dirty = $derived(content !== savedContent);
  let savedRecently = $derived(
    !dirty && lastSavedAt !== null && Date.now() - lastSavedAt < 2500,
  );

  onMount(async () => {
    try {
      const fetched = await getPrompt(kind);
      doc = fetched;
      content = fetched.content;
      savedContent = fetched.content;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  });

  $effect(() => {
    if (!doc) return;
    const _track = content;
    if (autosaveTimer) clearTimeout(autosaveTimer);
    if (!dirty) return;
    autosaveTimer = setTimeout(() => {
      void save();
    }, AUTOSAVE_DEBOUNCE_MS);
    return () => {
      if (autosaveTimer) {
        clearTimeout(autosaveTimer);
        autosaveTimer = null;
      }
    };
  });

  async function save() {
    if (!doc) return;
    if (saving) return;
    if (content === savedContent) return;
    saving = true;
    error = null;
    const snapshot = content;
    try {
      await savePrompt(kind, snapshot);
      savedContent = snapshot;
      lastSavedAt = Date.now();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      saving = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      void save();
    }
  }

  function insertAtCursor(token: string) {
    if (!textarea) return;
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const next = content.slice(0, start) + token + content.slice(end);
    content = next;
    queueMicrotask(() => {
      if (!textarea) return;
      const pos = start + token.length;
      textarea.focus();
      textarea.setSelectionRange(pos, pos);
    });
  }
</script>

<section class="prompt-editor" class:has-side-panel={doc?.supports_env_placeholders}>
  <header class="head">
    <div class="title-row">
      <h3 class="title">{doc?.label ?? "…"}</h3>
      {#if dirty}
        <span class="dot" title="Unsaved changes" aria-label="Unsaved changes"></span>
      {/if}
    </div>
    {#if doc}
      <span class="path" title={`Stored at: ${doc.path}`}>{doc.path}</span>
    {/if}
    {#if description}
      <p class="description">{description}</p>
    {/if}
  </header>

  <div class="body">
    <div class="editor-pane">
      <textarea
        bind:this={textarea}
        bind:value={content}
        onkeydown={handleKeydown}
        spellcheck="false"
        rows="14"
        placeholder="Loading…"
        disabled={!doc}
      ></textarea>
    </div>

    {#if doc?.supports_env_placeholders}
      <EnvPlaceholdersPanel onInsert={insertAtCursor} />
    {/if}
  </div>

  <footer class="foot">
    <div class="status">
      {#if error}
        <span class="error">Failed to save: {error}</span>
      {:else if saving}
        <span class="muted">Saving…</span>
      {:else if dirty}
        <span class="muted">Unsaved changes — autosave in progress.</span>
      {:else if savedRecently}
        <span class="ok"><Check size={ICON_SIZE.sm} /> Saved</span>
      {:else}
        <span class="muted">Cmd/Ctrl+S to save now.</span>
      {/if}
    </div>
    <Button variant="primary" disabled={!dirty || saving} onclick={save}>
      <Save size={ICON_SIZE.sm} />
      Save
    </Button>
  </footer>
</section>

<style>
  .prompt-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4);
    background: var(--surface-base);
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-md);
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .title {
    margin: 0;
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
  }

  .path {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--text-disabled);
  }

  .description {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .body {
    display: grid;
    grid-template-columns: 1fr;
    gap: var(--space-3);
  }

  .has-side-panel .body {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .editor-pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  textarea {
    width: 100%;
    min-height: 220px;
    padding: var(--space-3);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    line-height: 1.55;
    color: var(--text-primary);
    background: var(--surface-elevated);
    border: 1px solid var(--border-hard);
    border-radius: var(--radius-sm);
    resize: vertical;
  }

  textarea:focus {
    outline: none;
    border-color: var(--accent);
  }

  textarea:disabled {
    opacity: var(--opacity-disabled);
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .status {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--font-size-sm);
  }

  .muted {
    color: var(--text-muted);
  }

  .ok {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--success);
  }

  .error {
    color: var(--danger);
  }
</style>
