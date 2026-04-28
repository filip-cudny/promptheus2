<script lang="ts">
  import { History, SearchX, FilterX } from "lucide-svelte";

  let {
    variant,
    query,
    activeFiltersLabel,
    onClearFilters,
  }: {
    variant: "no-history" | "no-query-match" | "no-filter-match";
    query?: string;
    activeFiltersLabel?: string;
    onClearFilters?: () => void;
  } = $props();
</script>

<div class="empty-state" role="status" aria-live="polite">
  {#if variant === "no-history"}
    <span class="icon"><History size={32} /></span>
    <h2 class="title">No history yet</h2>
  {:else if variant === "no-query-match"}
    <span class="icon"><SearchX size={32} /></span>
    <h2 class="title">No matches for "{query ?? ""}"</h2>
    <p class="description">Try a shorter query or different keywords.</p>
    {#if onClearFilters}
      <button class="cta" onclick={onClearFilters}>Clear search</button>
    {/if}
  {:else}
    <span class="icon"><FilterX size={32} /></span>
    <h2 class="title">No history matches the active filters</h2>
    {#if activeFiltersLabel}
      <p class="description">Active: {activeFiltersLabel}</p>
    {/if}
    {#if onClearFilters}
      <button class="cta primary" onclick={onClearFilters}>Clear filters</button>
    {/if}
  {/if}
</div>

<style>
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 32px;
    color: #e0e0e0;
    text-align: center;
  }

  .icon {
    display: flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.35);
    margin-bottom: 4px;
  }

  .title {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.85);
  }

  .description {
    margin: 0;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.5);
    max-width: 360px;
  }

  .cta {
    margin-top: 8px;
    padding: 6px 16px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 6px;
    background: transparent;
    color: rgba(255, 255, 255, 0.85);
    cursor: pointer;
    font: inherit;
    font-size: 12px;
  }

  .cta:hover {
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(255, 255, 255, 0.35);
  }

  .cta.primary {
    background: rgba(100, 160, 255, 0.18);
    border-color: rgba(100, 160, 255, 0.5);
    color: rgba(180, 210, 255, 0.95);
  }

  .cta.primary:hover {
    background: rgba(100, 160, 255, 0.28);
    border-color: rgba(100, 160, 255, 0.7);
  }
</style>
