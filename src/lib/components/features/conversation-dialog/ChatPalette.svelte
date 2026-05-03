<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { ChevronRight, MessagesSquare, Plus, X } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { highlightFor } from "$lib/utils/highlightMatches";
  import { handleListNavKey } from "$lib/utils/listNavigation";
  import type { SearchResult } from "$lib/types/historySearch";
  import { useHistorySearch } from "$lib/stores/useHistorySearch.svelte";
  import CommandPalette from "$lib/components/shared/ui/CommandPalette.svelte";
  import KbdHint from "$lib/components/shared/ui/KbdHint.svelte";
  import { displayName, snippetFor, formatTimestamp } from "$lib/utils/historySearchSnippet";

  let { open, onClose, onNewChat, onOpenConversation }: {
    open: boolean;
    onClose: () => void;
    onNewChat: () => void;
    onOpenConversation: (entryId: string) => void;
  } = $props();

  const historySearch = useHistorySearch();

  let query = $state("");
  let highlightedIndex = $state(0);
  let inputEl = $state<HTMLInputElement | undefined>();
  let itemEls: (HTMLElement | null)[] = $state([]);

  $effect(() => {
    if (!open) return;
    const el = itemEls[highlightedIndex];
    if (el) el.scrollIntoView({ block: "nearest" });
  });

  type Item =
    | { kind: "new-chat" }
    | { kind: "recent"; result: SearchResult };

  let items = $derived<Item[]>([
    { kind: "new-chat" },
    ...historySearch.results.map<Item>((r) => ({ kind: "recent", result: r })),
  ]);

  $effect(() => {
    const _ = items.length;
    if (highlightedIndex >= items.length) {
      highlightedIndex = Math.max(0, items.length - 1);
    }
  });

  $effect(() => {
    if (open) {
      query = "";
      highlightedIndex = 0;
      historySearch.clear();
      historySearch.search("");
      tick().then(() => inputEl?.focus());
    } else {
      historySearch.cancel();
    }
  });

  $effect(() => {
    if (!open) return;
    historySearch.search(query);
  });

  function activate(item: Item) {
    if (item.kind === "new-chat") {
      onNewChat();
    } else {
      onOpenConversation(item.result.entry.id);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      const item = items[highlightedIndex];
      if (item) activate(item);
      return;
    }
    const nav = handleListNavKey(e, highlightedIndex, items.length);
    if (nav !== null) {
      e.preventDefault();
      e.stopPropagation();
      highlightedIndex = nav;
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
    historySearch.cancel();
  });

</script>

<CommandPalette
  {open}
  {onClose}
  bind:query
  bind:inputRef={inputEl}
  placeholder="Search or start a chat"
  variant="overlay"
>
  {#snippet headerExtras()}
    <button
      type="button"
      class="palette-close"
      aria-label="Close"
      onclick={onClose}
    >
      <X size={ICON_SIZE.md} />
    </button>
  {/snippet}
  {#snippet body()}
    <div class="palette-section-header">
      <span class="palette-section-icon"><ChevronRight size={ICON_SIZE.sm} /></span>
      <span>Quick actions</span>
    </div>
    {#each items as item, i (item.kind === "new-chat" ? "new-chat" : item.result.entry.id)}
      {#if item.kind === "new-chat"}
        <button
          bind:this={itemEls[i]}
          type="button"
          role="option"
          aria-selected={i === highlightedIndex}
          class="palette-item"
          class:highlight={i === highlightedIndex}
          onmouseenter={() => (highlightedIndex = i)}
          onclick={() => activate(item)}
        >
          <span class="palette-item-icon"><Plus size={ICON_SIZE.md} /></span>
          <span class="palette-item-name">New chat</span>
          <span class="palette-item-hint">Enter</span>
        </button>
      {:else}
        {#if i === 1}
          <div class="palette-section-header">
            <span class="palette-section-icon"><ChevronRight size={ICON_SIZE.sm} /></span>
            <span>Recents</span>
          </div>
        {/if}
        {@const snippet = snippetFor(item.result)}
        <button
          bind:this={itemEls[i]}
          type="button"
          role="option"
          aria-selected={i === highlightedIndex}
          class="palette-item"
          class:highlight={i === highlightedIndex}
          onmouseenter={() => (highlightedIndex = i)}
          onclick={() => activate(item)}
        >
          <span class="palette-item-icon"><MessagesSquare size={ICON_SIZE.md} /></span>
          <span class="palette-item-main">
            <span class="palette-item-name">
              {@html highlightFor(displayName(item.result), item.result.matches, ["title", "skill_name"])}
            </span>
            {#if snippet}
              <span class="palette-item-snippet">{@html highlightFor(snippet.text, snippet.matches, [snippet.field])}</span>
            {/if}
          </span>
          <span class="palette-item-hint">{formatTimestamp(item.result)}</span>
        </button>
      {/if}
    {/each}
    {#if items.length === 1 && !historySearch.isLoading}
      <div class="palette-empty">
        {query.trim() ? "No matching conversations" : "No recent conversations"}
      </div>
    {/if}
  {/snippet}
  {#snippet footer()}
    <span><KbdHint keys={["↑↓"]} /> / <KbdHint keys={["⌃JK"]} /> Navigate</span>
    <span><KbdHint keys={["↵"]} /> Open</span>
    <span><KbdHint keys={["esc"]} /> Close</span>
  {/snippet}
</CommandPalette>

<style>
  .palette-close {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--text-disabled);
    cursor: pointer;
    padding: var(--space-0) var(--space-6);
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
  }

  .palette-close:hover {
    color: var(--text-primary);
  }

  .palette-section-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-4) var(--space-7) var(--space-2);
    color: var(--text-disabled);
    font-size: var(--font-size-sm);
    text-transform: none;
  }

  .palette-section-icon {
    display: inline-flex;
    align-items: center;
    color: var(--text-disabled);
  }

  :global(.palette-item-main) {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }


  :global(.palette-item-main .palette-item-name) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.palette-item-snippet) {
    color: var(--text-disabled);
    font-size: var(--font-size-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.palette-item-hint) {
    color: var(--text-disabled);
    font-size: var(--font-size-sm);
    flex-shrink: 0;
  }

  :global(.palette-item-name mark),
  :global(.palette-item-snippet mark) {
    background: rgba(255, 220, 100, 0.25);
    color: inherit;
    padding: var(--space-0);
    border-radius: 2px;
  }
</style>
