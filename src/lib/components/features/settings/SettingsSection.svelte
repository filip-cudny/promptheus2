<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    hint,
    actions,
    body,
    footer,
    flush = false,
  }: {
    title?: string;
    hint?: string;
    actions?: Snippet;
    body: Snippet;
    footer?: Snippet;
    flush?: boolean;
  } = $props();

  const hasHeader = $derived(Boolean(title || hint || actions));
</script>

<section class="settings-section" class:flush>
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
      {#if actions}
        <div class="head-actions">
          {@render actions()}
        </div>
      {/if}
    </header>
  {/if}

  <div class="body" class:padded={!flush}>
    {@render body()}
  </div>

  {#if footer}
    <footer class="foot">
      {@render footer()}
    </footer>
  {/if}
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

  .head-actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-shrink: 0;
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .body.padded {
    padding: var(--space-8);
    gap: var(--space-8);
  }

  .foot {
    border-top: 1px solid var(--border-faint);
    padding: var(--space-4) var(--space-8);
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-3);
  }
</style>
