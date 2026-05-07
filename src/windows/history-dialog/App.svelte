<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { X } from "lucide-svelte";
  import { getHistoryStore } from "$lib/stores/history.svelte";
  import { getHistorySearchStore } from "$lib/stores/historySearch.svelte";
  import { openConversationDialog } from "$lib/services/conversationDialog";
  import { getUiState, setUiState } from "$lib/services/uiState";
  import type { HistoryEntry } from "$lib/types";
  import HistoryEntryRow from "$lib/components/features/history/HistoryEntryRow.svelte";
  import HistoryEmptyState from "$lib/components/features/history/HistoryEmptyState.svelte";
  import HistoryToolbar from "$lib/components/features/history/HistoryToolbar.svelte";
  import HistoryPagination from "$lib/components/features/history/HistoryPagination.svelte";
  import { formatActiveFilters, getActiveFilterChips } from "$lib/utils/historyFilters";

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
  let entriesListEl = $state<HTMLDivElement | null>(null);
  let scrolled = $state(false);

  let totalPages = $derived(Math.max(1, Math.ceil(searchStore.total / pageSize)));
  let pageResults = $derived(searchStore.results);
  let chips = $derived(getActiveFilterChips(searchStore));

  let showSkeletons = $derived(searchStore.loading && pageResults.length === 0);
  let showProgressBar = $derived(searchStore.loading && pageResults.length > 0);
  let showChipBar = $derived(chips.length > 0 || (searchStore.total > 0 && !showSkeletons));

  let emptyVariant = $derived.by<"no-history" | "no-query-match" | "no-filter-match" | null>(() => {
    if (searchStore.results.length > 0) return null;
    if (searchStore.loading) return null;
    if (store.entries.length === 0) return "no-history";
    if (searchStore.query.trim() !== "") return "no-query-match";
    if (
      searchStore.typeFilter !== "all" ||
      searchStore.statusFilter !== "all" ||
      searchStore.skillFilter.size > 0 ||
      searchStore.timeRange !== "all"
    ) {
      return "no-filter-match";
    }
    return "no-history";
  });

  let skeletonCount = $derived(Math.min(pageSize, 5));

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
    searchStore.timeRange;
    currentPage = 0;
  });

  $effect(() => {
    store.entries.length;
    searchStore.refresh();
    searchStore.refreshSkills();
  });

  function getRowButtons(): HTMLButtonElement[] {
    if (!entriesListEl) return [];
    return Array.from(entriesListEl.querySelectorAll<HTMLButtonElement>(".entry-row"));
  }

  function focusedRowIndex(): number {
    const rows = getRowButtons();
    for (let i = 0; i < rows.length; i++) {
      if (rows[i] === document.activeElement) return i;
    }
    return -1;
  }

  function focusRow(idx: number) {
    const rows = getRowButtons();
    if (rows.length === 0) return;
    const clamped = Math.max(0, Math.min(rows.length - 1, idx));
    rows[clamped]?.focus();
  }

  function isInSearchInput(target: EventTarget | null): boolean {
    return !!searchInput && target === searchInput;
  }

  function handleWindowKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") {
      e.preventDefault();
      searchInput?.focus();
      searchInput?.select();
      return;
    }

    if (e.key === "PageDown") {
      if (currentPage < totalPages - 1) {
        e.preventDefault();
        currentPage++;
      }
      return;
    }
    if (e.key === "PageUp") {
      if (currentPage > 0) {
        e.preventDefault();
        currentPage--;
      }
      return;
    }

    const inSearch = isInSearchInput(e.target);
    const rowIdx = focusedRowIndex();
    const onRow = rowIdx >= 0;

    if (inSearch && e.key === "ArrowDown") {
      if (pageResults.length > 0) {
        e.preventDefault();
        focusRow(0);
      }
      return;
    }

    if (onRow) {
      if (e.key === "ArrowDown" || e.key === "j") {
        e.preventDefault();
        focusRow(rowIdx + 1);
      } else if (e.key === "ArrowUp" || e.key === "k") {
        e.preventDefault();
        if (rowIdx === 0) searchInput?.focus();
        else focusRow(rowIdx - 1);
      } else if (e.key === "Escape") {
        e.preventDefault();
        searchInput?.focus();
      } else if (e.key === "Home") {
        e.preventDefault();
        focusRow(0);
      } else if (e.key === "End") {
        e.preventDefault();
        focusRow(getRowButtons().length - 1);
      }
    }
  }

  function handleListScroll() {
    if (!entriesListEl) return;
    const next = entriesListEl.scrollTop > 0;
    if (next !== scrolled) scrolled = next;
  }

  $effect(() => {
    pageResults;
    currentPage;
    if (entriesListEl) entriesListEl.scrollTop = 0;
    if (scrolled) scrolled = false;
  });

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
  <header class="topbar" class:scrolled>
    <HistoryToolbar {searchStore} bind:searchInput />

    {#if showChipBar}
      <div class="chip-bar">
        <div class="chips">
          {#each chips as chip (chip.key)}
            <button class="chip" type="button" onclick={chip.remove} aria-label={`Remove filter: ${chip.label}`}>
              <span class="chip-label">{chip.label}</span>
              <X size={10} />
            </button>
          {/each}
        </div>
        {#if searchStore.total > 0}
          <span class="result-count">
            {searchStore.total}
            {searchStore.total === 1 ? "result" : "results"}
          </span>
        {/if}
      </div>
    {/if}

    <div class="progress-line" class:visible={showProgressBar} aria-hidden="true">
      <span class="progress-bar"></span>
    </div>
  </header>

  <span aria-live="polite" aria-atomic="true" class="sr-only">
    {searchStore.loading
      ? "Searching"
      : `${searchStore.total} ${searchStore.total === 1 ? "result" : "results"}`}
  </span>

  <div class="entries-list" bind:this={entriesListEl} onscroll={handleListScroll}>
    {#if showSkeletons}
      {#each Array(skeletonCount) as _, i (i)}
        <div class="skeleton-row" aria-hidden="true">
          <div class="skeleton-icon"></div>
          <div class="skeleton-body">
            <div class="skeleton-line skeleton-line-title"></div>
            <div class="skeleton-line skeleton-line-text"></div>
          </div>
        </div>
      {/each}
    {:else}
      {#each pageResults as result (result.entry.id)}
        <HistoryEntryRow
          entry={result.entry}
          matches={result.matches}
          onOpen={handleOpen}
          oncopy={(content) => navigator.clipboard.writeText(content)}
        />
      {/each}
      {#if emptyVariant}
        <HistoryEmptyState
          variant={emptyVariant}
          query={searchStore.query}
          activeFiltersLabel={formatActiveFilters(searchStore)}
          onClearFilters={() => searchStore.clear()}
        />
      {/if}
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

  .topbar {
    position: sticky;
    top: 0;
    z-index: var(--z-sticky);
    background: var(--surface-base);
    border-bottom: 1px solid var(--border-faint);
    flex-shrink: 0;
    transition: box-shadow var(--motion-default) var(--ease-default),
      border-color var(--motion-default) var(--ease-default);
  }

  .topbar.scrolled {
    border-bottom-color: var(--border-default);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.18);
  }

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

  .progress-line {
    position: absolute;
    left: 0;
    right: 0;
    bottom: -1px;
    height: 2px;
    overflow: hidden;
    pointer-events: none;
    opacity: 0;
    transition: opacity var(--motion-fast) var(--ease-default);
  }

  .progress-line.visible {
    opacity: 1;
  }

  .progress-bar {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      transparent 0%,
      var(--accent) 50%,
      transparent 100%
    );
    transform: translateX(-100%);
    animation: progress-slide 1.1s var(--ease-default) infinite;
  }

  @keyframes progress-slide {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(100%);
    }
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

  .skeleton-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-6);
    background: var(--surface-base);
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-xl);
    max-width: 760px;
    width: 100%;
    margin-inline: auto;
  }

  .skeleton-icon {
    width: 14px;
    height: 14px;
    border-radius: var(--radius-sm);
    background: var(--surface-overlay-faint);
    flex-shrink: 0;
    margin-top: 3px;
    animation: skeleton-pulse 1.4s ease-in-out infinite;
  }

  .skeleton-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .skeleton-line {
    height: 10px;
    border-radius: var(--radius-sm);
    background: var(--surface-overlay-faint);
    animation: skeleton-pulse 1.4s ease-in-out infinite;
  }

  .skeleton-line-title {
    width: 38%;
    max-width: 240px;
  }

  .skeleton-line-text {
    width: 72%;
    max-width: 460px;
  }

  .skeleton-row:nth-child(2) .skeleton-line,
  .skeleton-row:nth-child(2) .skeleton-icon {
    animation-delay: 0.08s;
  }
  .skeleton-row:nth-child(3) .skeleton-line,
  .skeleton-row:nth-child(3) .skeleton-icon {
    animation-delay: 0.16s;
  }
  .skeleton-row:nth-child(4) .skeleton-line,
  .skeleton-row:nth-child(4) .skeleton-icon {
    animation-delay: 0.24s;
  }
  .skeleton-row:nth-child(5) .skeleton-line,
  .skeleton-row:nth-child(5) .skeleton-icon {
    animation-delay: 0.32s;
  }

  @keyframes skeleton-pulse {
    0%, 100% { opacity: 0.55; }
    50% { opacity: 1; }
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
