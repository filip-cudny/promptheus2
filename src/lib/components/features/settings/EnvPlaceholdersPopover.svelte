<script lang="ts">
  import { onMount } from "svelte";
  import FloatingPanel from "$lib/components/shared/ui/FloatingPanel.svelte";
  import {
    getEnvironmentPlaceholders,
    type EnvPlaceholder,
  } from "$lib/services/prompts";

  let {
    visible,
    anchorEl,
    onInsert,
    onclose,
  }: {
    visible: boolean;
    anchorEl: HTMLElement | undefined;
    onInsert: (token: string) => void;
    onclose: () => void;
  } = $props();

  let placeholders = $state<EnvPlaceholder[]>([]);
  let error = $state<string | null>(null);
  let loaded = $state(false);

  async function ensureLoaded() {
    if (loaded || error) return;
    try {
      placeholders = await getEnvironmentPlaceholders();
      loaded = true;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(() => {
    void ensureLoaded();
  });

  $effect(() => {
    if (visible) void ensureLoaded();
  });
</script>

<FloatingPanel {visible} {anchorEl} position="below" fitContent {onclose}>
  <div class="popover">
    <div class="header">
      <span class="title">Placeholders</span>
      <span class="hint">Click to insert at cursor</span>
    </div>

    {#if error}
      <p class="error">Failed to load: {error}</p>
    {:else if !loaded}
      <p class="empty">Loading…</p>
    {:else if placeholders.length === 0}
      <p class="empty">No placeholders available.</p>
    {:else}
      <ul class="list">
        {#each placeholders as p (p.token)}
          <li>
            <button
              type="button"
              class="chip"
              title={`${p.description}\nExample: ${p.example}`}
              onclick={() => onInsert(p.token)}
            >
              <code class="token">{p.token}</code>
              <span class="label">{p.label}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</FloatingPanel>

<style>
  .popover {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-width: 240px;
    max-width: 320px;
  }

  .header {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .title {
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: var(--tracking-label);
    color: var(--text-disabled);
  }

  .hint {
    font-size: var(--font-size-xs);
    color: var(--text-muted);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .chip {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    transition: background var(--motion-fast) var(--ease-default),
      border-color var(--motion-fast) var(--ease-default);
  }

  .chip:hover,
  .chip:focus-visible {
    background: var(--surface-overlay-faint);
    border-color: var(--border-faint);
    outline: none;
  }

  .token {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--accent);
  }

  .label {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }

  .empty,
  .error {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .error {
    color: var(--danger);
  }
</style>
