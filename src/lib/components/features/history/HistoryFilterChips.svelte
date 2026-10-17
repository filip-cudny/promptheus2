<script lang="ts">
  import { X } from "lucide-svelte";
  import type { ActiveFilterChip } from "$lib/utils/historyFilters";

  let { chips, total }: { chips: ActiveFilterChip[]; total: number } = $props();
</script>

<div class="chip-bar">
  <div class="chips">
    {#each chips as chip (chip.key)}
      <button class="chip" type="button" onclick={chip.remove} aria-label={`Remove filter: ${chip.label}`}>
        <span class="chip-label">{chip.label}</span>
        <X size={10} />
      </button>
    {/each}
  </div>
  {#if total > 0}
    <span class="result-count">
      {total}
      {total === 1 ? "result" : "results"}
    </span>
  {/if}
</div>

<style>
  .chip-bar {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: 0 var(--space-6) var(--space-4);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    flex: 1;
    min-width: 0;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 2px var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-full);
    background: var(--surface-overlay-faint);
    color: var(--text-secondary);
    cursor: pointer;
    font: inherit;
    font-size: var(--font-size-sm);
    line-height: 1.4;
    transition: background var(--motion-fast) var(--ease-default),
      color var(--motion-fast) var(--ease-default),
      border-color var(--motion-fast) var(--ease-default);
  }

  .chip:hover {
    background: var(--surface-overlay);
    color: var(--text-primary);
    border-color: var(--border-default);
  }

  .chip:focus-visible {
    outline: 2px solid var(--accent-ring);
    outline-offset: 1px;
  }

  .chip-label {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .result-count {
    flex-shrink: 0;
    color: var(--text-muted);
    font-size: var(--font-size-sm);
    letter-spacing: var(--tracking-label);
  }
</style>
