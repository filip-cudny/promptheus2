<script lang="ts">
  import { Sparkles } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import FloatingPanel from "$lib/components/shared/ui/FloatingPanel.svelte";
  import KbdHint from "$lib/components/shared/ui/KbdHint.svelte";
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
  let focused = $state(false);
  let inputEl: HTMLInputElement | undefined = $state();

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
    bind:this={inputEl}
    onkeydown={handleKeydown}
    onfocus={() => (focused = true)}
    onblur={() => (focused = false)}
  />
  <!-- FloatingPanel places the list and caps its height once on open and does not follow the input. Correct while the sidebar does not scroll and the results do not grow while it is open; it breaks when the section list is taller than the window and is scrolled, or a query opened with few rows is shortened to many. Then close it on the sidebar's scroll and re-place it on a change of results. -->
  <FloatingPanel
    visible={listed && focused}
    anchorEl={inputEl}
    fitContent
    onclose={() => (focused = false)}
  >
    <ul class="results" role="presentation" onmousedown={(e) => e.preventDefault()}>
      {#each results as entry, i (entry.key)}
        <li>
          <button
            class="result"
            class:active={i === highlightedIndex}
            onmouseenter={() => (highlightedIndex = i)}
            onclick={() => choose(entry)}
          >
            <span class="label">{entry.label}</span>
            <span class="detail">
              {sectionLabel(entry.section)}{#if entry.description} · {entry.description}{/if}
            </span>
          </button>
        </li>
      {:else}
        <li class="empty">No matching settings</li>
      {/each}
      <li class="ask-item">
        <button
          class="result ask"
          class:active={highlightedIndex === askIndex}
          onmouseenter={() => (highlightedIndex = askIndex)}
          onclick={askAgent}
        >
          <Sparkles size={ICON_SIZE.md} />
          <span class="question">Ask agent: {query.trim()}</span>
          {#if highlightedIndex === askIndex}<KbdHint keys={["Enter"]} />{/if}
        </button>
      </li>
    </ul>
  </FloatingPanel>
</div>

<style>
  .search {
    margin: var(--space-0) var(--space-8) var(--space-4);
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
    width: 300px;
    box-sizing: border-box;
    list-style: none;
    margin: var(--space-0);
    padding: var(--space-0) var(--space-2);
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
    color: var(--text-primary);
    font: inherit;
    font-size: var(--font-size-base);
    cursor: pointer;
  }

  .result.active {
    background: var(--surface-overlay);
  }

  .detail {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .ask-item {
    margin-top: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border-faint);
  }

  .result.ask {
    flex-direction: row;
    align-items: center;
    gap: var(--space-4);
  }

  .result.ask.active {
    background: var(--accent-bg-soft);
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

  .empty {
    padding: 6px var(--space-4);
    font-size: var(--font-size-xs);
    color: var(--text-faint);
  }
</style>
