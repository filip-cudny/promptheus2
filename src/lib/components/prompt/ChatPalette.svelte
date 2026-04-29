<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { error as logError } from "@tauri-apps/plugin-log";
  import { ChevronRight, MessagesSquare, Plus, X } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { extractSkillDisplayText } from "$lib/utils/skillDisplay";
  import { highlightFor, truncateAroundMatch } from "$lib/utils/highlightMatches";
  import type { FieldMatch, SearchField, SearchResult, SearchResponse } from "$lib/types/historySearch";

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
    if (e.key === "ArrowDown" || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "j")) {
      e.preventDefault();
      e.stopPropagation();
      highlightedIndex = Math.min(items.length - 1, highlightedIndex + 1);
      return;
    }
    if (e.key === "ArrowUp" || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k")) {
      e.preventDefault();
      e.stopPropagation();
      highlightedIndex = Math.max(0, highlightedIndex - 1);
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown, true);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown, true);
    cancelInflight();
  });

  function displayName(r: SearchResult): string {
    const e = r.entry;
    return extractSkillDisplayText(e.title ?? e.skill_name ?? "Chat");
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
      ? (e.input_content_rendered ?? e.input_content ?? "")
      : (e.output_content_rendered ?? e.output_content ?? "");
    if (!raw) return null;
    const truncated = truncateAroundMatch(extractSkillDisplayText(raw), r.matches, source, 80);
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

{#if open}
  <div class="palette-root">
    <button
      type="button"
      aria-label="Close palette"
      class="palette-scrim"
      onclick={onClose}
    ></button>
    <div class="palette-modal palette-modal-enter" role="dialog" aria-modal="true">
      <div class="palette-header">
        <input
          bind:this={inputEl}
          bind:value={query}
          oninput={() => (highlightedIndex = 0)}
          class="palette-input"
          type="text"
          placeholder="Search or start a chat"
          autocomplete="off"
          spellcheck="false"
        />
        <button
          type="button"
          class="palette-close"
          aria-label="Close"
          onclick={onClose}
        >
          <X size={ICON_SIZE.md} />
        </button>
      </div>

      <div class="palette-body" role="listbox">
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
      </div>

      <div class="palette-footer">
        <span><kbd>↑↓</kbd> / <kbd>⌃JK</kbd> Navigate</span>
        <span><kbd>↵</kbd> Open</span>
        <span><kbd>esc</kbd> Close</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .palette-root {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 80px;
    z-index: 1000;
  }

  .palette-scrim {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    border: 0;
    padding: 0;
    cursor: default;
    animation: palette-scrim-enter 140ms ease-out both;
  }

  .palette-modal {
    position: relative;
    width: min(640px, 86%);
    background: #252525;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    color: #e0e0e0;
  }

  .palette-modal-enter {
    animation: palette-modal-enter 140ms ease-out both;
  }

  @keyframes palette-scrim-enter {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes palette-modal-enter {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .palette-header {
    display: flex;
    align-items: center;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .palette-input {
    flex: 1;
    appearance: none;
    border: 0;
    background: transparent;
    color: #fff;
    font: inherit;
    font-size: 14px;
    padding: 12px 14px;
    outline: none;
  }

  .palette-input::placeholder {
    color: rgba(255, 255, 255, 0.35);
  }

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

  .palette-body {
    display: flex;
    flex-direction: column;
    max-height: 360px;
    overflow-y: auto;
    padding: 4px 0;
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

  .palette-item {
    appearance: none;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.85);
    font: inherit;
    text-align: left;
    padding: 8px 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .palette-item.highlight {
    background: rgba(255, 255, 255, 0.08);
  }

  .palette-item-icon {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.7);
  }

  .palette-item-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .palette-item-name {
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .palette-item-snippet {
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .palette-item-hint {
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
    flex-shrink: 0;
  }

  .palette-empty {
    color: rgba(255, 255, 255, 0.4);
    padding: 16px;
    text-align: center;
    font-size: 12px;
  }

  .palette-footer {
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    padding: 6px 14px;
    display: flex;
    gap: 12px;
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
  }

  .palette-footer kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 3px;
    padding: 1px 5px;
    font-family: inherit;
    font-size: 10px;
    line-height: 1;
    color: rgba(255, 255, 255, 0.7);
    margin-right: 4px;
    vertical-align: middle;
  }

  :global(.palette-item-name mark),
  :global(.palette-item-snippet mark) {
    background: rgba(255, 220, 100, 0.25);
    color: inherit;
    padding: 0;
    border-radius: 2px;
  }
</style>
