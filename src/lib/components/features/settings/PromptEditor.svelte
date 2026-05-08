<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Braces } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import {
    getPrompt,
    savePrompt,
    type PromptDoc,
    type PromptKind,
  } from "$lib/services/prompts";
  import { useSaveTracker } from "$lib/stores/saveTracker.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import SaveStatusIndicator from "$lib/components/shared/widgets/SaveStatusIndicator.svelte";
  import HighlightedTextarea from "$lib/components/shared/ui/HighlightedTextarea.svelte";
  import EnvPlaceholdersPopover from "./EnvPlaceholdersPopover.svelte";

  let {
    kind,
    description,
  }: {
    kind: PromptKind;
    description?: string;
  } = $props();

  const settingsStore = getSettingsStore();

  let doc = $state<PromptDoc | null>(null);
  let content = $state("");
  let savedContent = $state("");
  let textarea = $state<HTMLTextAreaElement | undefined>(undefined);
  let placeholdersBtn = $state<HTMLButtonElement | undefined>(undefined);
  let placeholdersOpen = $state(false);

  const tracker = useSaveTracker({
    debounceMs: settingsStore.settings?.autosave_debounce_ms ?? 1000,
  });

  const PLACEHOLDER_RE = /\{\{[^{}\n]+\}\}/g;

  const highlightPattern = $derived(
    doc?.supports_env_placeholders ? PLACEHOLDER_RE : undefined,
  );

  onMount(() => {
    void load();
    tracker.attachKeyboard(window);
    tracker.attachBeforeUnload(window);
  });

  onDestroy(() => {
    tracker.destroy();
  });

  async function load() {
    try {
      const fetched = await getPrompt(kind);
      doc = fetched;
      content = fetched.content;
      savedContent = fetched.content;
    } catch (e) {
      tracker.setError(e instanceof Error ? e.message : String(e));
    }
  }

  $effect(() => {
    if (!doc) return;
    const _track = content;
    if (content === savedContent) {
      tracker.cancel();
      return;
    }
    tracker.scheduleSave(persist);
  });

  async function persist() {
    if (!doc) return;
    if (content === savedContent) return;
    const snapshot = content;
    await savePrompt(kind, snapshot);
    savedContent = snapshot;
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

<section class="prompt-editor">
  <header class="head">
    <div class="title-row">
      <h3 class="title">{doc?.label ?? "…"}</h3>
      <SaveStatusIndicator {tracker} />

      <div class="head-actions">
        {#if doc?.supports_env_placeholders}
          <button
            type="button"
            class="icon-btn"
            class:active={placeholdersOpen}
            bind:this={placeholdersBtn}
            onclick={() => (placeholdersOpen = !placeholdersOpen)}
            title="Insert placeholder"
            aria-label="Insert placeholder"
          >
            <Braces size={ICON_SIZE.sm} />
          </button>
        {/if}
        {#if doc?.path}
          <details class="path-details">
            <summary title="Show file path">…</summary>
            <code class="path">{doc.path}</code>
          </details>
        {/if}
      </div>
    </div>
    {#if description}
      <p class="description">{description}</p>
    {/if}
  </header>

  <div class="editor-pane">
    <HighlightedTextarea
      bind:value={content}
      {highlightPattern}
      placeholder={doc ? "" : "Loading…"}
      disabled={!doc}
      onTextareaRef={(el) => (textarea = el)}
    />
  </div>

  {#if doc?.supports_env_placeholders}
    <EnvPlaceholdersPopover
      visible={placeholdersOpen}
      anchorEl={placeholdersBtn}
      onclose={() => (placeholdersOpen = false)}
      onInsert={(token) => {
        insertAtCursor(token);
        placeholdersOpen = false;
      }}
    />
  {/if}
</section>

<style>
  .prompt-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-height: 0;
    flex: 1;
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

  .head-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    transition: background var(--motion-fast) var(--ease-default),
      color var(--motion-fast) var(--ease-default),
      border-color var(--motion-fast) var(--ease-default);
  }

  .icon-btn:hover {
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
  }

  .icon-btn.active {
    background: var(--accent-bg-soft);
    color: var(--accent);
    border-color: var(--accent-border);
  }

  .path-details {
    position: relative;
    font-size: var(--font-size-xs);
  }

  .path-details summary {
    list-style: none;
    cursor: pointer;
    color: var(--text-faint);
    padding: 0 var(--space-2);
    border-radius: var(--radius-sm);
    user-select: none;
  }

  .path-details summary::-webkit-details-marker {
    display: none;
  }

  .path-details summary:hover {
    color: var(--text-muted);
    background: var(--surface-overlay-faint);
  }

  .path-details[open] summary {
    color: var(--text-muted);
  }

  .path-details code {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: var(--space-1);
    padding: var(--space-2) var(--space-3);
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--text-muted);
    background: var(--surface-floating-popover);
    border: 1px solid var(--surface-floating-popover-border);
    border-radius: var(--radius-sm);
    white-space: nowrap;
    box-shadow: var(--shadow-md);
    z-index: var(--z-overlay);
  }

  .description {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .editor-pane {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }
</style>
