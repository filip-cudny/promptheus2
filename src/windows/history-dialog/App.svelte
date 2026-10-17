<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getHistorySearchStore } from "$lib/stores/historySearch.svelte";
  import { openConversationDialog } from "$lib/services/conversationDialog";
  import { copyHistoryEntryContent, searchHistory } from "$lib/services/history";
  import { onHistoryChanged } from "$lib/services/events";
  import { getUiState, setUiState } from "$lib/services/uiState";
  import type { HistoryEntry } from "$lib/types";
  import { error as logError } from "@tauri-apps/plugin-log";
  import HistoryEntryRow from "$lib/components/features/history/HistoryEntryRow.svelte";
  import HistoryEmptyState from "$lib/components/features/history/HistoryEmptyState.svelte";
  import HistoryToolbar from "$lib/components/features/history/HistoryToolbar.svelte";
  import HistoryPagination from "$lib/components/features/history/HistoryPagination.svelte";
  import HistoryFilterChips from "$lib/components/features/history/HistoryFilterChips.svelte";
  import { useHistoryAudio } from "$lib/components/features/history/drivers/useHistoryAudio.svelte";
  import { useHistoryKeyboard } from "$lib/components/features/history/drivers/useHistoryKeyboard.svelte";
  import { formatActiveFilters, getActiveFilterChips } from "$lib/utils/historyFilters";

  const PAGE_SIZES = [10, 25, 50] as const;
  const PAGE_SIZE_KEY = "history-dialog.page_size";

  let pageSize = $state(10);
  let currentPage = $state(0);

  const searchStore = getHistorySearchStore({
    pageSize: () => pageSize,
    currentPage: () => currentPage,
  });

  let unlisten: (() => void) | undefined;
  let historyEmpty = $state(false);

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
    if (historyEmpty) return "no-history";
    if (searchStore.query.trim() !== "") return "no-query-match";
    if (searchStore.hasActiveFilters) return "no-filter-match";
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

  async function refreshHistoryEmpty() {
    try {
      const { total } = await searchHistory({
        query: "",
        type_filter: "all",
        status_filter: "all",
        limit: 1,
        offset: 0,
      });
      historyEmpty = total === 0;
    } catch (e) {
      logError(`history emptiness check failed: ${e}`);
    }
  }

  function refreshAll() {
    void refreshHistoryEmpty();
    void searchStore.refresh();
    void searchStore.refreshSkills();
  }

  const audio = useHistoryAudio({ entries: () => pageResults.map((r) => r.entry) });

  function stepPage(delta: 1 | -1): boolean {
    const next = currentPage + delta;
    if (next < 0 || next >= totalPages) return false;
    currentPage = next;
    return true;
  }

  useHistoryKeyboard({
    searchInput: () => searchInput,
    entriesList: () => entriesListEl,
    hasResults: () => pageResults.length > 0,
    stepPage,
  });

  function handleListScroll() {
    if (!entriesListEl) return;
    const next = entriesListEl.scrollTop > 0;
    if (next !== scrolled) scrolled = next;
  }

  $effect(() => {
    currentPage;
    if (entriesListEl) entriesListEl.scrollTop = 0;
    scrolled = false;
  });

  onMount(async () => {
    const saved = await getUiState<number>(PAGE_SIZE_KEY);
    if (saved && PAGE_SIZES.includes(saved as (typeof PAGE_SIZES)[number])) {
      pageSize = saved;
    }
    searchInput?.focus();

    unlisten = await onHistoryChanged(refreshAll);
    refreshAll();
  });

  onDestroy(() => unlisten?.());

  async function handleOpen(entry: HistoryEntry) {
    await openConversationDialog(entry.skill_id ?? "", entry.title ?? entry.skill_name ?? "", entry.id);
  }

  async function handleCopy(entryId: string) {
    try {
      await copyHistoryEntryContent(entryId);
    } catch (e) {
      logError("Failed to copy history entry: " + e);
    }
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
      <HistoryFilterChips {chips} total={searchStore.total} />
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
          audioClip={audio.audioClips[result.entry.id] ?? null}
          retrying={audio.retryingIds.has(result.entry.id)}
          onOpen={handleOpen}
          oncopy={handleCopy}
          onRetry={audio.retry}
          onExportAudio={audio.exportAudio}
          onDiscardAudio={audio.discardAudio}
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
    position: relative;
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
</style>
