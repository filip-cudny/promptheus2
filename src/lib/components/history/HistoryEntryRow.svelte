<script lang="ts">
  import { Mic, MessageSquare, MessagesSquare, CircleAlert, SquareArrowOutUpRight, Copy, Check } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import type { HistoryEntry } from "$lib/types";

  let { entry, onOpen }: {
    entry: HistoryEntry;
    onOpen: (entry: HistoryEntry) => void;
  } = $props();

  let isChat = $derived(!entry.quick_action);
  let isTranscription = $derived(
    entry.entry_type === "speech" && !entry.skill_name && entry.quick_action,
  );

  let copied = $state(false);
  let copyTimeout: ReturnType<typeof setTimeout> | null = null;

  function copyToClipboard(e: MouseEvent) {
    e.stopPropagation();
    const text = entry.output_content ?? entry.input_content;
    navigator.clipboard.writeText(text);
    copied = true;
    if (copyTimeout) clearTimeout(copyTimeout);
    copyTimeout = setTimeout(() => { copied = false; }, 1500);
  }

  let turnCount = $derived(
    isChat && entry.conversation_data?.nodes?.length
      ? Math.floor(entry.conversation_data.nodes.length / 2)
      : null,
  );

  let displayName = $derived(
    entry.title ?? entry.skill_name ?? (entry.entry_type === "speech" ? "Transcription" : "Unknown"),
  );

  let inputPreview = $derived(truncate(entry.input_content, 120));

  function truncate(text: string, max: number): string {
    if (!text || text.length <= max) return text ?? "";
    return text.slice(0, max) + "…";
  }

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
  onclick={() => { if (!isTranscription) onOpen(entry); }}
  title={!entry.success && entry.error ? `Error: ${entry.error}` : isTranscription ? "" : "Open conversation"}
>
  <div class="entry-icon" class:icon-chat={isChat} class:icon-quick={!isChat}>
    {#if isChat}
      <MessagesSquare size={ICON_SIZE.md} />
    {:else if entry.entry_type === "speech"}
      <Mic size={ICON_SIZE.md} />
    {:else}
      <MessageSquare size={ICON_SIZE.md} />
    {/if}
    {#if !entry.success}
      <span class="error-badge">
        <CircleAlert size={ICON_SIZE.sm} />
      </span>
    {/if}
  </div>

  <div class="entry-body">
    <div class="entry-header">
      <span class="prompt-name">{displayName}</span>
      {#if turnCount}
        <span class="turn-count">({turnCount} turns)</span>
      {/if}
      <span class="timestamp">{formatTimestamp(entry)}</span>
      {#if isTranscription}
        <button class="copy-btn" class:copied onclick={copyToClipboard} title="Copy transcription">
          {#if copied}
            <Check size={ICON_SIZE.md} />
          {:else}
            <Copy size={ICON_SIZE.md} />
          {/if}
        </button>
      {:else}
        <span class="open-icon">
          <SquareArrowOutUpRight size={ICON_SIZE.md} />
        </span>
      {/if}
    </div>
    {#if inputPreview}
      <div class="input-preview">{inputPreview}</div>
    {/if}
  </div>
</button>

<style>
  .entry-row {
    display: flex;
    align-items: flex-start;
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

  .entry-icon {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    margin-top: 2px;
  }

  .icon-chat {
    color: rgba(255, 255, 255, 0.4);
  }

  .icon-quick {
    color: rgba(100, 160, 255, 0.85);
  }

  .error-badge {
    color: #ff6b6b;
    display: flex;
    align-items: center;
  }

  .entry-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .entry-header {
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

  .input-preview {
    color: rgba(255, 255, 255, 0.35);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .timestamp {
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
    flex-shrink: 0;
    white-space: nowrap;
    margin-left: auto;
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

  .copy-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    padding: 2px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.3);
    cursor: pointer;
  }

  .copy-btn:hover {
    color: rgba(255, 255, 255, 0.7);
    background: rgba(255, 255, 255, 0.08);
  }

  .copy-btn.copied {
    color: rgba(80, 200, 120, 0.9);
  }
</style>
