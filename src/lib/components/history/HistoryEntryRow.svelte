<script lang="ts">
  import { Mic, MessageSquare, MessagesSquare, CircleAlert, SquareArrowOutUpRight } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import type { HistoryEntry } from "$lib/types";

  let { entry, onOpen }: {
    entry: HistoryEntry;
    onOpen: (entry: HistoryEntry) => void;
  } = $props();

  let turnCount = $derived(
    entry.conversation_data?.nodes?.length
      ? Math.floor(entry.conversation_data.nodes.length / 2)
      : null,
  );

  function formatTimestamp(entry: HistoryEntry): string {
    const raw = entry.updated_at ?? entry.created_at ?? entry.timestamp;
    const date = new Date(raw);
    if (isNaN(date.getTime())) return raw;
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMin = Math.floor(diffMs / 60000);
    if (diffMin < 1) return "Just now";
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHours = Math.floor(diffMin / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    return date.toLocaleDateString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  }
</script>

<button
  class="entry-row"
  class:error={!entry.success}
  onclick={() => onOpen(entry)}
  title={!entry.success && entry.error ? `Error: ${entry.error}` : "Open conversation"}
>
  <div class="entry-icons">
    {#if entry.entry_type === "speech"}
      <Mic size={ICON_SIZE.md} />
    {:else}
      <MessageSquare size={ICON_SIZE.md} />
    {/if}
    {#if entry.is_multi_turn}
      <span class="conversation-badge">
        <MessagesSquare size={ICON_SIZE.sm} />
      </span>
    {/if}
    {#if !entry.success}
      <span class="error-badge">
        <CircleAlert size={ICON_SIZE.sm} />
      </span>
    {/if}
  </div>

  <div class="entry-info">
    <span class="prompt-name">{entry.prompt_name ?? "Unknown"}</span>
    {#if turnCount}
      <span class="turn-count">({turnCount} turns)</span>
    {/if}
  </div>

  <span class="timestamp">{formatTimestamp(entry)}</span>

  <span class="open-icon">
    <SquareArrowOutUpRight size={ICON_SIZE.md} />
  </span>
</button>

<style>
  .entry-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: #2a2a2a;
    border: 1px solid #3a3a3a;
    border-radius: 8px;
    color: #e0e0e0;
    cursor: pointer;
    width: 100%;
    text-align: left;
    font: inherit;
  }

  .entry-row:hover {
    background: #333;
    border-color: #4a4a4a;
  }

  .entry-row.error {
    background: rgba(255, 80, 80, 0.08);
    border-color: rgba(255, 80, 80, 0.25);
  }

  .entry-row.error:hover {
    background: rgba(255, 80, 80, 0.14);
  }

  .entry-icons {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    color: rgba(255, 255, 255, 0.5);
  }

  .conversation-badge {
    color: rgba(100, 160, 255, 0.85);
    display: flex;
    align-items: center;
  }

  .error-badge {
    color: #ff6b6b;
    display: flex;
    align-items: center;
  }

  .entry-info {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .prompt-name {
    font-weight: 600;
    color: #fff;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .turn-count {
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
    flex-shrink: 0;
  }

  .timestamp {
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
    flex-shrink: 0;
    white-space: nowrap;
  }

  .open-icon {
    display: flex;
    align-items: center;
    color: rgba(255, 255, 255, 0.3);
    flex-shrink: 0;
  }

  .entry-row:hover .open-icon {
    color: rgba(255, 255, 255, 0.7);
  }
</style>
