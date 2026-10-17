<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    count,
    actions,
  }: {
    title: string;
    count?: number;
    actions: Snippet;
  } = $props();
</script>

<div class="section-header">
  <span class="header-label">
    {title}
    {#if count !== undefined}
      <span class="badge">{count}</span>
    {/if}
  </span>
  <span class="header-actions">
    {@render actions()}
  </span>
</div>

<style>
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--space-2) var(--space-6);
    color: var(--text-muted);
    font-size: var(--font-size-sm);
    text-transform: capitalize;
    letter-spacing: var(--tracking-label);
    box-sizing: border-box;
  }

  .header-label {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .badge {
    background: rgba(255, 255, 255, 0.15);
    border-radius: var(--radius-xl);
    padding: var(--space-0) var(--space-3);
    font-size: var(--font-size-xs);
    line-height: 16px;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }
</style>
