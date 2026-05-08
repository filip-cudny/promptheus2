<script lang="ts">
  import { onMount } from "svelte";
  import { Braces } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import HighlightedTextarea from "$lib/components/shared/ui/HighlightedTextarea.svelte";
  import EnvPlaceholdersPopover from "./EnvPlaceholdersPopover.svelte";
  import {
    getEnvironmentPlaceholders,
    type EnvPlaceholder,
  } from "$lib/services/prompts";

  let {
    value = $bindable<string>(""),
    filePath,
    error = null,
  }: {
    value?: string;
    filePath: string;
    error?: string | null;
  } = $props();

  let textarea = $state<HTMLTextAreaElement | undefined>(undefined);
  let placeholdersBtn = $state<HTMLButtonElement | undefined>(undefined);
  let placeholdersOpen = $state(false);
  let knownTokens = $state<Set<string>>(new Set());

  const PLACEHOLDER_RE = /\{\{[a-z_][a-z0-9_]*\}\}/g;

  onMount(async () => {
    try {
      const list: EnvPlaceholder[] = await getEnvironmentPlaceholders();
      knownTokens = new Set(list.map((p) => p.token));
    } catch {}
  });

  function isKnown(token: string): boolean {
    return knownTokens.has(token);
  }

  function insertAtCursor(token: string) {
    if (!textarea) return;
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const next = value.slice(0, start) + token + value.slice(end);
    value = next;
    queueMicrotask(() => {
      if (!textarea) return;
      const pos = start + token.length;
      textarea.focus();
      textarea.setSelectionRange(pos, pos);
    });
  }

  const charCount = $derived(value.length);
  const tokenCount = $derived.by(() => {
    const re = new RegExp(PLACEHOLDER_RE.source, "g");
    let m: RegExpExecArray | null;
    let count = 0;
    while ((m = re.exec(value)) !== null) {
      count += 1;
      if (m[0].length === 0) re.lastIndex += 1;
    }
    return count;
  });
</script>

<section class="card body-card" class:error>
  <header class="head">
    <div class="title-row">
      <h3>Body</h3>
      <p class="hint">Wrapped in <code>&lt;skill&gt;…&lt;/skill&gt;</code> when invoked.</p>
    </div>
    <div class="actions">
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
      <details class="path-details">
        <summary title="Show file path">…</summary>
        <code class="path">{filePath}</code>
      </details>
    </div>
  </header>

  <div class="editor-pane">
    <HighlightedTextarea
      bind:value
      highlightPattern={PLACEHOLDER_RE}
      unknownTokenChecker={isKnown}
      placeholder="Write the skill prompt body here…"
      minHeight={320}
      onTextareaRef={(el) => (textarea = el)}
    />
  </div>

  <footer class="foot">
    <span class="counter">{charCount} chars · {tokenCount} placeholders</span>
    {#if error}
      <span class="err">{error}</span>
    {/if}
  </footer>
</section>

<EnvPlaceholdersPopover
  visible={placeholdersOpen}
  anchorEl={placeholdersBtn}
  onclose={() => (placeholdersOpen = false)}
  onInsert={(token) => {
    insertAtCursor(token);
    placeholdersOpen = false;
  }}
/>

<style>
  .card {
    background: var(--surface-base);
    border: 1px solid var(--border-hard);
    border-radius: var(--radius-lg);
    padding: var(--space-6) var(--space-7);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .card.error {
    border-color: var(--danger);
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .title-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }

  h3 {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--text-muted);
    margin: 0;
  }

  .hint {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-disabled);
  }

  .hint code {
    font-family: var(--font-mono);
    font-size: 0.9em;
    padding: 0 4px;
    background: var(--surface-overlay-faint);
    border-radius: var(--radius-sm);
  }

  .actions {
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

  .editor-pane {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-3);
    font-size: var(--font-size-xs);
    color: var(--text-faint);
  }

  .err {
    color: var(--danger);
    font-size: var(--font-size-sm);
  }
</style>
