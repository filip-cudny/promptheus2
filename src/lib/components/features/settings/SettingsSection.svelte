<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    hint,
    body,
  }: {
    title?: string;
    hint?: string;
    body: Snippet;
  } = $props();

  const hasHeader = $derived(Boolean(title || hint));
</script>

<section class="settings-section">
  {#if hasHeader}
    <header class="head">
      <div class="head-text">
        {#if title}
          <h2>{title}</h2>
        {/if}
        {#if hint}
          <p class="hint">{hint}</p>
        {/if}
      </div>
    </header>
  {/if}

  <div class="body">
    {@render body()}
  </div>
</section>

<style>
  .settings-section {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    background: var(--surface-base);
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-6);
    padding: var(--space-8) var(--space-8) var(--space-6);
    border-bottom: 1px solid var(--border-faint);
  }

  .head-text {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }

  h2 {
    margin: 0;
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
  }

  .hint {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: var(--space-8);
    gap: var(--space-8);
  }
</style>
