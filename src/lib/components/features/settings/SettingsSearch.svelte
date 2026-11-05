<script lang="ts">
  import { getSettingsCatalogStore } from "$lib/stores/settingsCatalog.svelte";
  import { getSettingsNavigation } from "$lib/stores/settingsNavigation.svelte";
  import { searchCatalog } from "$lib/utils/settingsSearch";
  import type { CatalogEntry, CatalogSection } from "$lib/types";

  let { sectionLabel }: { sectionLabel: (section: CatalogSection) => string } = $props();

  const catalog = getSettingsCatalogStore();
  const navigation = getSettingsNavigation();

  let query = $state("");
  let highlightedIndex = $state(0);

  const results = $derived(searchCatalog(catalog.entries, query));

  $effect(() => {
    query;
    highlightedIndex = 0;
  });

  function choose(entry: CatalogEntry) {
    query = "";
    navigation.navigateTo(entry.key);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      query = "";
    } else if (e.key === "ArrowDown" && results.length > 0) {
      e.preventDefault();
      e.stopPropagation();
      highlightedIndex = (highlightedIndex + 1) % results.length;
    } else if (e.key === "ArrowUp" && results.length > 0) {
      e.preventDefault();
      e.stopPropagation();
      highlightedIndex = (highlightedIndex - 1 + results.length) % results.length;
    } else if (e.key === "Enter" && results[highlightedIndex]) {
      e.preventDefault();
      e.stopPropagation();
      choose(results[highlightedIndex]);
    }
  }
</script>

<div class="search">
  <input
    type="search"
    placeholder="Search settings"
    bind:value={query}
    onkeydown={handleKeydown}
  />
  {#if query.trim() !== ""}
    <ul class="results">
      {#each results as entry, i (entry.key)}
        <li>
          <button
            class="result"
            class:active={i === highlightedIndex}
            onmouseenter={() => (highlightedIndex = i)}
            onclick={() => choose(entry)}
          >
            <span class="label">{entry.label}</span>
            <span class="section">{sectionLabel(entry.section)}</span>
          </button>
        </li>
      {:else}
        <li class="empty">No matching settings</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .search {
    margin: var(--space-0) var(--space-8) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  input {
    width: 100%;
    box-sizing: border-box;
    padding: 6px var(--space-4);
    background: var(--surface-overlay-faint);
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font: inherit;
    font-size: var(--font-size-base);
  }

  input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .results {
    list-style: none;
    margin: var(--space-0);
    padding: var(--space-0);
    display: flex;
    flex-direction: column;
  }

  .result {
    display: flex;
    flex-direction: column;
    width: 100%;
    text-align: left;
    padding: 6px var(--space-4);
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-base);
    cursor: pointer;
  }

  .result.active {
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
  }

  .section {
    font-size: var(--font-size-xs);
    color: var(--text-faint);
  }

  .empty {
    padding: 6px var(--space-4);
    font-size: var(--font-size-xs);
    color: var(--text-faint);
  }
</style>
