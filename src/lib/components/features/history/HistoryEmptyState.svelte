<script lang="ts">
  import { History, SearchX, FilterX } from "lucide-svelte";
  import EmptyState from "$lib/components/shared/ui/EmptyState.svelte";
  import Button from "$lib/components/shared/ui/Button.svelte";

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

<div class="history-empty-state" role="status" aria-live="polite">
  {#if variant === "no-history"}
    <EmptyState title="No history yet">
      {#snippet icon()}<History size={32} />{/snippet}
    </EmptyState>
  {:else if variant === "no-query-match"}
    {#if onClearFilters}
      <EmptyState
        title='No matches for "{query ?? ""}"'
        description="Try a shorter query or different keywords."
      >
        {#snippet icon()}<SearchX size={32} />{/snippet}
        {#snippet action()}
          <Button variant="ghost" onclick={onClearFilters}>Clear search</Button>
        {/snippet}
      </EmptyState>
    {:else}
      <EmptyState
        title='No matches for "{query ?? ""}"'
        description="Try a shorter query or different keywords."
      >
        {#snippet icon()}<SearchX size={32} />{/snippet}
      </EmptyState>
    {/if}
  {:else if onClearFilters}
    <EmptyState
      title="No history matches the active filters"
      description={activeFiltersLabel ? `Active: ${activeFiltersLabel}` : undefined}
    >
      {#snippet icon()}<FilterX size={32} />{/snippet}
      {#snippet action()}
        <Button variant="primary" onclick={onClearFilters}>Clear filters</Button>
      {/snippet}
    </EmptyState>
  {:else}
    <EmptyState
      title="No history matches the active filters"
      description={activeFiltersLabel ? `Active: ${activeFiltersLabel}` : undefined}
    >
      {#snippet icon()}<FilterX size={32} />{/snippet}
    </EmptyState>
  {/if}
</div>

<style>
  .history-empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
  }


</style>
