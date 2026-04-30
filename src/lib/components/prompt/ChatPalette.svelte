<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { error as logError } from "@tauri-apps/plugin-log";
  import { ChevronRight, MessagesSquare, Plus, X } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { highlightFor, truncateAroundMatch } from "$lib/utils/highlightMatches";
  import { handleListNavKey } from "$lib/utils/listNavigation";
  import type { FieldMatch, SearchField, SearchResult, SearchResponse } from "$lib/types/historySearch";
  import CommandPalette from "$lib/components/ui/CommandPalette.svelte";

  let { open, onClose, onNewChat, onOpenConversation }: {
    open: boolean;
    onClose: () => void;
    onNewChat: () => void;
    onOpenConversation: (entryId: string) => void;
  } = $props();

  const RESULT_LIMIT = 30;
  const DEBOUNCE_MS = 120;

  let query = $state("");
  let results = $state<SearchResult[]>([]);
  let loading = $state(false);
  let highlightedIndex = $state(0);
  let inputEl = $state<HTMLInputElement | undefined>();
  let itemEls: (HTMLElement | null)[] = $state([]);

  $effect(() => {
    if (!open) return;
    const el = itemEls[highlightedIndex];
    if (el) el.scrollIntoView({ block: "nearest" });
  });

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let inflight: AbortController | null = null;

  type Item =
    | { kind: "new-chat" }
    | { kind: "recent"; result: SearchResult };

  let items = $derived<Item[]>([
    { kind: "new-chat" },
    ...results.map<Item>((r) => ({ kind: "recent", result: r })),
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
      results = [];
      runSearchSoon(0);
      tick().then(() => inputEl?.focus());
    } else {
      cancelInflight();
    }
  });

  $effect(() => {
    if (!open) return;
    query;
    runSearchSoon(query.trim() === "" ? 0 : DEBOUNCE_MS);
  });

  function cancelInflight() {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
    if (inflight) {
      inflight.abort();
      inflight = null;
    }
  }

  function runSearchSoon(wait: number) {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(runSearch, wait);
  }

  async function runSearch() {
    if (inflight) inflight.abort();
    const ac = new AbortController();
    inflight = ac;
    loading = true;
    try {
      const response = await invoke<SearchResponse>("search_history", {
        query: {
          query: query.trim(),
          type_filter: "chat",
          status_filter: "all",
          skill_ids: [],
          date_from: null,
          limit: RESULT_LIMIT,
          offset: 0,
        },
      });
      if (!ac.signal.aborted) {
        results = response.results;
      }
    } catch (e) {
      if (!ac.signal.aborted) {
        logError(`search_history (chat-palette) failed: ${e}`);
      }
    } finally {
      if (inflight === ac) inflight = null;
      if (!ac.signal.aborted) loading = false;
    }
  }

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
    cancelInflight();
  });

  function displayName(r: SearchResult): string {
    const e = r.entry;
    return e.title ?? e.skill_name ?? "Chat";
  }

  function snippetSourceFor(matches: readonly FieldMatch[]): SearchField | null {
    const input = matches.find((m) => m.field === "input_content" && m.indices.length > 0);
    if (input) return "input_content";
    const output = matches.find((m) => m.field === "output_content" && m.indices.length > 0);
    if (output) return "output_content";
    return null;
  }

  function snippetFor(r: SearchResult): { field: SearchField; text: string; matches: FieldMatch[] } | null {
    const source = snippetSourceFor(r.matches);
    if (!source) return null;
    const e = r.entry;
    const raw = source === "input_content"
      ? (e.input_content ?? "")
      : (e.output_content ?? "");
    if (!raw) return null;
    const truncated = truncateAroundMatch(raw, r.matches, source, 80);
    if (!truncated.text) return null;
    return { field: source, text: truncated.text, matches: truncated.matches };
  }

  function formatTimestamp(r: SearchResult): string {
    const e = r.entry;
    const raw = e.updated_at ?? e.created_at ?? e.timestamp;
    const date = new Date(raw);
    if (isNaN(date.getTime())) return raw;
    const now = new Date();
    const startOfToday = new Date(now);
    startOfToday.setHours(0, 0, 0, 0);
    if (date.getTime() >= startOfToday.getTime()) return "Today";
    const diffDays = Math.floor((startOfToday.getTime() - date.getTime()) / 86400000);
    if (diffDays < 1) return "Yesterday";
    if (diffDays < 7) return `${diffDays + 1}d ago`;
    return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }
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
    {#if items.length === 1 && !loading}
      <div class="palette-empty">
        {query.trim() ? "No matching conversations" : "No recent conversations"}
      </div>
    {/if}
  {/snippet}
  {#snippet footer()}
    <span><kbd>↑↓</kbd> / <kbd>⌃JK</kbd> Navigate</span>
    <span><kbd>↵</kbd> Open</span>
    <span><kbd>esc</kbd> Close</span>
  {/snippet}
</CommandPalette>

<style>
  .palette-close {
    appearance: none;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.4);
    cursor: pointer;
    padding: 0 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
  }

  .palette-close:hover {
    color: rgba(255, 255, 255, 0.85);
  }

  .palette-section-header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px 14px 4px;
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
    text-transform: none;
  }

  .palette-section-icon {
    display: inline-flex;
    align-items: center;
    color: rgba(255, 255, 255, 0.35);
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
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.palette-item-hint) {
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
    flex-shrink: 0;
  }

  :global(.palette-item-name mark),
  :global(.palette-item-snippet mark) {
    background: rgba(255, 220, 100, 0.25);
    color: inherit;
    padding: 0;
    border-radius: 2px;
  }
</style>
