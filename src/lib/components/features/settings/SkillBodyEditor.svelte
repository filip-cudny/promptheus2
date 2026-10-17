<script lang="ts">
  import { onMount } from "svelte";
  import { Braces } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import HighlightedTextarea from "$lib/components/shared/ui/HighlightedTextarea.svelte";
  import FilePathReveal from "$lib/components/shared/ui/FilePathReveal.svelte";
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

  let textarea = $state<ReturnType<typeof HighlightedTextarea> | undefined>(undefined);
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
        class="editor-icon-btn"
        class:active={placeholdersOpen}
        bind:this={placeholdersBtn}
        onclick={() => (placeholdersOpen = !placeholdersOpen)}
        title="Insert placeholder"
        aria-label="Insert placeholder"
      >
        <Braces size={ICON_SIZE.sm} />
      </button>
      <FilePathReveal path={filePath} />
    </div>
  </header>

  <div class="editor-pane">
    <HighlightedTextarea
      bind:this={textarea}
      bind:value
      highlightPattern={PLACEHOLDER_RE}
      unknownTokenChecker={isKnown}
      placeholder="Write the skill prompt body here…"
      minHeight={320}
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
    textarea?.insert(token);
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
