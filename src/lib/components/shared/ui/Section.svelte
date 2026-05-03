<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    actions,
    children,
  }: {
    title?: string;
    actions?: Snippet;
    children: Snippet;
  } = $props();
</script>

{#if title || actions}
  <div class="section-header">
    {#if title}<span class="section-title">{title}</span>{/if}
    {#if actions}<div class="section-actions">{@render actions()}</div>{/if}
  </div>
{/if}
<div class="section-body">
  {@render children()}
</div>

<style>
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--border-faint);
  }

  .section-title {
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: var(--tracking-label);
    color: var(--text-disabled);
  }

  .section-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

</style>
