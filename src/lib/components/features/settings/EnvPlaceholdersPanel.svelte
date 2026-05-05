<script lang="ts">
  import { onMount } from "svelte";
  import { getEnvironmentPlaceholders, type EnvPlaceholder } from "$lib/services/prompts";

  let {
    onInsert,
  }: {
    onInsert: (token: string) => void;
  } = $props();

  let placeholders = $state<EnvPlaceholder[]>([]);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      placeholders = await getEnvironmentPlaceholders();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  });
</script>

<aside class="env-placeholders">
  <div class="header">
    <span class="title">Placeholders</span>
    <span class="hint">Click to insert</span>
  </div>
  {#if error}
    <p class="error">Failed to load placeholders: {error}</p>
  {:else if placeholders.length === 0}
    <p class="empty">Loading…</p>
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
</aside>

<style>
  .env-placeholders {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4);
    background: var(--surface-elevated);
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-md);
    min-width: 220px;
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
    gap: var(--space-2);
  }

  .chip {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-1);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: var(--surface-base);
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    transition: background var(--motion-fast) var(--ease-default),
      border-color var(--motion-fast) var(--ease-default);
  }

  .chip:hover {
    background: var(--surface-overlay);
    border-color: var(--border-hard);
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
