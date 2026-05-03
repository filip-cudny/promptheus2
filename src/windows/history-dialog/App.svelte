<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getHistoryStore } from "$lib/stores/history.svelte";
  import { getHistorySearchStore } from "$lib/stores/historySearch.svelte";
  import { openConversationDialog } from "$lib/services/conversationDialog";
  import { getUiState, setUiState } from "$lib/services/uiState";
  import type { HistoryEntry } from "$lib/types";
  import HistoryEntryRow from "$lib/components/features/history/HistoryEntryRow.svelte";
  import HistoryEmptyState from "$lib/components/features/history/HistoryEmptyState.svelte";
  import HistoryToolbar from "$lib/components/features/history/HistoryToolbar.svelte";
  import HistoryPagination from "$lib/components/features/history/HistoryPagination.svelte";
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
    searchInput?.focus();
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
    <HistoryPagination
      pageSizes={PAGE_SIZES}
      {pageSize}
      bind:currentPage
      {totalPages}
      onChangePageSize={changePageSize}
    />
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
