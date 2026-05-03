<script lang="ts">
  let { keys }: { keys: string[] } = $props();
</script>

{#each keys as key, i}
  {#if i > 0}<span class="sep">+</span>{/if}
  <kbd>{key}</kbd>
{/each}

<style>
  kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-0) var(--space-1);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm);
    font-family: var(--font-sans);
    font-size: var(--font-size-xs);
    line-height: var(--line-height-tight);
    color: var(--text-muted);
    background: var(--surface-overlay-faint);
  }

  .sep {
    color: var(--text-faint);
    margin: 0 var(--space-1);
    font-size: var(--font-size-xs);
  }
</style>
