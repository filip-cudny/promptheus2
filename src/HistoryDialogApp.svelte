<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { ChevronLeft, ChevronRight } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { getHistoryStore } from "$lib/stores/history.svelte";
  import { getHistorySearchStore } from "$lib/stores/historySearch.svelte";
  import { openConversationDialog } from "$lib/services/conversationDialog";
  import { getUiState, setUiState } from "$lib/services/uiState";
  import type { HistoryEntry } from "$lib/types";
  import HistoryEntryRow from "$lib/components/history/HistoryEntryRow.svelte";
  import HistoryToolbar from "$lib/components/history/HistoryToolbar.svelte";

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

  $effect(() => {
    if (currentPage >= totalPages) {
      currentPage = Math.max(0, totalPages - 1);
    }
  });

  $effect(() => {
    searchStore.query;
    searchStore.typeFilter;
    searchStore.statusFilter;
    currentPage = 0;
  });

  $effect(() => {
    store.entries.length;
    searchStore.refresh();
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

  <div class="entries-list" class:loading={searchStore.loading && pageResults.length === 0}>
    {#each pageResults as result (result.entry.id)}
      <HistoryEntryRow entry={result.entry} matches={result.matches} onOpen={handleOpen} />
    {:else}
      <div class="empty-state">
        {searchStore.hasActiveFilters ? "No matches" : "No history yet"}
      </div>
    {/each}
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
        <button
          class="nav-btn"
          disabled={currentPage === 0}
          onclick={() => currentPage--}
          title="Previous page"
        >
          <ChevronLeft size={ICON_SIZE.md} />
        </button>
        <button
          class="nav-btn"
          disabled={currentPage >= totalPages - 1}
          onclick={() => currentPage++}
          title="Next page"
        >
          <ChevronRight size={ICON_SIZE.md} />
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .dialog-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #1e1e1e;
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 13px;
    overflow: hidden;
  }

  .entries-list {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    transition: opacity 120ms ease;
  }

  .entries-list.loading {
    opacity: 0.5;
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.35);
    font-size: 14px;
  }

  .pagination-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    flex-shrink: 0;
  }

  .page-size {
    display: flex;
    gap: 2px;
  }

  .page-size-btn {
    padding: 2px 8px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
    font: inherit;
    font-size: 11px;
  }

  .page-size-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.8);
  }

  .page-size-btn.active {
    background: rgba(100, 160, 255, 0.15);
    border-color: rgba(100, 160, 255, 0.4);
    color: rgba(100, 160, 255, 0.9);
  }

  .page-label {
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
  }

  .page-nav {
    display: flex;
    gap: 2px;
  }

  .nav-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
  }

  .nav-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.8);
  }

  .nav-btn:disabled {
    color: rgba(255, 255, 255, 0.15);
    cursor: default;
  }
</style>
