<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { ChevronLeft, ChevronRight } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import { getHistoryStore } from "$lib/stores/history.svelte";
  import { getHistorySearchStore } from "$lib/stores/historySearch.svelte";
  import { openConversationDialog } from "$lib/services/conversationDialog";
  import { getUiState, setUiState } from "$lib/services/uiState";
  import type { HistoryEntry } from "$lib/types";
  import HistoryEntryRow from "$lib/components/history/HistoryEntryRow.svelte";
  import HistoryEmptyState from "$lib/components/history/HistoryEmptyState.svelte";
  import HistoryToolbar from "$lib/components/history/HistoryToolbar.svelte";
  import { formatActiveFilters } from "$lib/utils/historyFilters";

  const PAGE_SIZES = [10, 25, 50] as const;
  const PAGE_SIZE_KEY = "history-dialog.page_size";

  const store = getHistoryStore();

  let pageSize = $state(10);
  let currentPage = $state(0);

  const searchStore = getHistorySearchStore({
    pageSize: () => pageSize,
    currentPage: () => currentPage,
  });

  let searchInput = $state<HTMLInputElement | null>(null);

  let totalPages = $derived(Math.max(1, Math.ceil(searchStore.total / pageSize)));
  let pageResults = $derived(searchStore.results);

  let emptyVariant = $derived.by<"no-history" | "no-query-match" | "no-filter-match" | null>(() => {
    if (searchStore.results.length > 0) return null;
    if (store.entries.length === 0) return "no-history";
    if (searchStore.query.trim() !== "") return "no-query-match";
    if (
      searchStore.typeFilter !== "all" ||
      searchStore.statusFilter !== "all" ||
      searchStore.skillFilter.size > 0
    ) {
      return "no-filter-match";
    }
    return "no-history";
  });

  $effect(() => {
    if (currentPage >= totalPages) {
      currentPage = Math.max(0, totalPages - 1);
    }
  });

  $effect(() => {
    searchStore.query;
    searchStore.typeFilter;
    searchStore.statusFilter;
    searchStore.skillFilter;
    currentPage = 0;
  });

  $effect(() => {
    store.entries.length;
    searchStore.refresh();
    searchStore.refreshSkills();
  });

  function handleWindowKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") {
      e.preventDefault();
      searchInput?.focus();
      searchInput?.select();
    }
  }

  onMount(async () => {
    await store.init();
    const saved = await getUiState<number>(PAGE_SIZE_KEY);
    if (saved && PAGE_SIZES.includes(saved as (typeof PAGE_SIZES)[number])) {
      pageSize = saved;
    }
    window.addEventListener("keydown", handleWindowKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleWindowKeydown);
    store.destroy();
  });

  async function handleOpen(entry: HistoryEntry) {
    await openConversationDialog(entry.skill_id ?? "", entry.title ?? entry.skill_name ?? "", entry.id);
  }

  async function changePageSize(size: number) {
    pageSize = size;
    currentPage = 0;
    await setUiState(PAGE_SIZE_KEY, size);
  }
</script>

<div class="dialog-shell">
  <HistoryToolbar {searchStore} bind:searchInput />

  <span aria-live="polite" aria-atomic="true" class="sr-only">
    {searchStore.loading
      ? "Searching"
      : `${searchStore.total} ${searchStore.total === 1 ? "result" : "results"}`}
  </span>

  <div class="entries-list" class:loading={searchStore.loading && pageResults.length === 0}>
    {#each pageResults as result (result.entry.id)}
      <HistoryEntryRow entry={result.entry} matches={result.matches} onOpen={handleOpen} oncopy={(content) => navigator.clipboard.writeText(content)} />
    {/each}
    {#if emptyVariant}
      <HistoryEmptyState
        variant={emptyVariant}
        query={searchStore.query}
        activeFiltersLabel={formatActiveFilters(searchStore)}
        onClearFilters={() => searchStore.clear()}
      />
    {/if}
  </div>

  {#if searchStore.total > 0}
    <div class="pagination-bar">
      <div class="page-size">
        {#each PAGE_SIZES as size}
          <button
            class="page-size-btn"
            class:active={pageSize === size}
            onclick={() => changePageSize(size)}
          >
            {size}
          </button>
        {/each}
      </div>

      <span class="page-label">Page {currentPage + 1} of {totalPages}</span>

      <div class="page-nav">
        <ActionIconButton
          icon={ChevronLeft}
          size={ICON_SIZE.md}
          disabled={currentPage === 0}
          onclick={() => currentPage--}
          title="Previous page"
        />
        <ActionIconButton
          icon={ChevronRight}
          size={ICON_SIZE.md}
          disabled={currentPage >= totalPages - 1}
          onclick={() => currentPage++}
          title="Next page"
        />
      </div>
    </div>
  {/if}
</div>

<style>
  .dialog-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--surface-base);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--font-size-base);
    overflow: hidden;
  }

  .entries-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    transition: opacity var(--motion-fast) var(--ease-default);
  }

  .entries-list.loading {
    opacity: 0.5;
  }

  .pagination-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-6);
    border-top: 1px solid var(--border-default);
    flex-shrink: 0;
  }

  .page-size {
    display: flex;
    gap: var(--space-1);
  }

  .page-size-btn {
    padding: var(--space-1) var(--space-4);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    font: inherit;
    font-size: var(--font-size-sm);
  }

  .page-size-btn:hover {
    background: var(--surface-overlay);
    color: var(--text-secondary);
  }

  .page-size-btn.active {
    background: var(--accent-bg-soft);
    border-color: var(--accent-bg);
    color: var(--accent);
  }

  .page-label {
    color: var(--text-disabled);
    font-size: var(--font-size-sm);
  }

  .page-nav {
    display: flex;
    gap: var(--space-1);
  }

  .page-nav :global(.action-icon-btn) {
    border: 1px solid var(--border-strong);
    padding: var(--space-1);
  }

  .page-size-btn:focus-visible {
    outline: 2px solid var(--accent-ring);
    outline-offset: 1px;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: var(--space-0);
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
