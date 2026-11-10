<script lang="ts">
  import { Sparkles } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { agentHasUsableModel } from "$lib/utils/agentModel";
  import { getSettingsCatalogStore } from "$lib/stores/settingsCatalog.svelte";
  import { getSettingsNavigation } from "$lib/stores/settingsNavigation.svelte";
  import { searchCatalog } from "$lib/utils/settingsSearch";
  import type { CatalogEntry, CatalogSection } from "$lib/types";

  let { sectionLabel }: { sectionLabel: (section: CatalogSection) => string } = $props();

  const catalog = getSettingsCatalogStore();
  const navigation = getSettingsNavigation();
  const agent = getSettingsAgent();
  const settingsStore = getSettingsStore();

  let query = $state("");
  let highlightedIndex = $state(0);

  const results = $derived(searchCatalog(catalog.entries, query));
  const listed = $derived(query.trim() !== "");
  const askIndex = $derived(results.length);

  $effect(() => {
    query;
    highlightedIndex = 0;
  });

  function choose(entry: CatalogEntry) {
    query = "";
    navigation.navigateTo(entry.key);
  }

  async function askAgent() {
    const question = query.trim();
    query = "";
    await agent.openPanel("search-row");
    const usable =
      settingsStore.settings !== null &&
      agentHasUsableModel(settingsStore.settings.surfaces, settingsStore.models);
    if (usable && !agent.running) void agent.send(question);
    else agent.fillDraft(question);
  }

  function chooseHighlighted() {
    if (highlightedIndex === askIndex) void askAgent();
    else choose(results[highlightedIndex]);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      query = "";
    } else if (e.key === "ArrowDown" && listed) {
      e.preventDefault();
      e.stopPropagation();
      highlightedIndex = (highlightedIndex + 1) % (askIndex + 1);
    } else if (e.key === "ArrowUp" && listed) {
      e.preventDefault();
      e.stopPropagation();
      highlightedIndex = (highlightedIndex - 1 + askIndex + 1) % (askIndex + 1);
    } else if (e.key === "Enter" && listed) {
      e.preventDefault();
      e.stopPropagation();
      chooseHighlighted();
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
  {#if listed}
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
      <li>
        <button
          class="result ask"
          class:active={highlightedIndex === askIndex}
          onmouseenter={() => (highlightedIndex = askIndex)}
          onclick={askAgent}
        >
          <Sparkles size={ICON_SIZE.md} />
          <span class="question">Ask agent: {query.trim()}</span>
          {#if highlightedIndex === askIndex}<kbd>Enter</kbd>{/if}
        </button>
      </li>
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

  .result.ask {
    flex-direction: row;
    align-items: center;
    gap: var(--space-4);
  }

  .result.ask :global(svg) {
    flex-shrink: 0;
    color: var(--accent);
  }

  .question {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  kbd {
    font: inherit;
    font-size: var(--font-size-xs);
    color: var(--text-faint);
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
