<script lang="ts">
  import { Search, X } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import type { HistorySearchStore } from "$lib/stores/historySearch.svelte";
  import type {
    HistoryStatusFilter,
    HistoryTypeFilter,
  } from "$lib/types/historySearch";

  let {
    searchStore,
    searchInput = $bindable<HTMLInputElement | null>(null),
  }: {
    searchStore: HistorySearchStore;
    searchInput?: HTMLInputElement | null;
  } = $props();

  const TYPE_OPTIONS: ReadonlyArray<{ value: HistoryTypeFilter; label: string }> = [
    { value: "all", label: "All" },
    { value: "chat", label: "Chat" },
    { value: "quick_action", label: "Quick Action" },
    { value: "speech", label: "Speech" },
  ];

  const STATUS_OPTIONS: ReadonlyArray<{ value: HistoryStatusFilter; label: string }> = [
    { value: "all", label: "All statuses" },
    { value: "success", label: "Success only" },
    { value: "error", label: "Errors only" },
  ];

  function handleSearchKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && searchStore.query !== "") {
      e.preventDefault();
      searchStore.query = "";
    }
  }

  function clearSearch() {
    searchStore.query = "";
    searchInput?.focus();
  }
</script>

<div class="toolbar">
  <div class="search-wrap">
    <span class="search-icon">
      <Search size={ICON_SIZE.md} />
    </span>
    <input
      bind:this={searchInput}
      bind:value={searchStore.query}
      onkeydown={handleSearchKeydown}
      type="search"
      class="search-input"
      placeholder="Search history..."
      aria-label="Search history"
      autocomplete="off"
      spellcheck="false"
    />
    {#if searchStore.query !== ""}
      <button class="clear-btn" onclick={clearSearch} title="Clear search" aria-label="Clear search">
        <X size={ICON_SIZE.md} />
      </button>
    {/if}
  </div>

  <div class="filters-row">
    <div class="type-segment" role="group" aria-label="Filter by type">
      {#each TYPE_OPTIONS as option (option.value)}
        <button
          class="segment-btn"
          class:active={searchStore.typeFilter === option.value}
          aria-pressed={searchStore.typeFilter === option.value}
          onclick={() => (searchStore.typeFilter = option.value)}
        >
          {option.label}
        </button>
      {/each}
    </div>

    <select
      class="status-select"
      aria-label="Filter by status"
      bind:value={searchStore.statusFilter}
    >
      {#each STATUS_OPTIONS as option (option.value)}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    flex-shrink: 0;
  }

  .search-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 10px;
    display: flex;
    align-items: center;
    color: rgba(255, 255, 255, 0.4);
    pointer-events: none;
  }

  .search-input {
    flex: 1;
    width: 100%;
    padding: 6px 32px 6px 30px;
    background: #2a2a2a;
    border: 1px solid #3a3a3a;
    border-radius: 6px;
    color: #e0e0e0;
    font: inherit;
    font-size: 13px;
    outline: none;
  }

  .search-input::-webkit-search-decoration,
  .search-input::-webkit-search-cancel-button {
    appearance: none;
  }

  .search-input:focus {
    border-color: rgba(100, 160, 255, 0.5);
    background: #2e2e2e;
  }

  .search-input::placeholder {
    color: rgba(255, 255, 255, 0.3);
  }

  .clear-btn {
    position: absolute;
    right: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.4);
    cursor: pointer;
  }

  .clear-btn:hover {
    color: rgba(255, 255, 255, 0.8);
    background: rgba(255, 255, 255, 0.08);
  }

  .filters-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .type-segment {
    display: flex;
    gap: 2px;
  }

  .segment-btn {
    padding: 3px 10px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.55);
    cursor: pointer;
    font: inherit;
    font-size: 11px;
  }

  .segment-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.85);
  }

  .segment-btn.active {
    background: rgba(100, 160, 255, 0.15);
    border-color: rgba(100, 160, 255, 0.4);
    color: rgba(100, 160, 255, 0.95);
  }

  .status-select {
    margin-left: auto;
    padding: 3px 8px;
    background: #2a2a2a;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    color: rgba(255, 255, 255, 0.75);
    font: inherit;
    font-size: 11px;
    cursor: pointer;
    outline: none;
  }

  .status-select:focus {
    border-color: rgba(100, 160, 255, 0.4);
  }
</style>
