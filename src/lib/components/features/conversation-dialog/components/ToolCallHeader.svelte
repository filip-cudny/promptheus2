<script lang="ts">
  import type { ToolCall, ToolCallType } from "$lib/types/ai";
  import {
    Search,
    Play,
    FileText,
    FilePen,
    Link,
    Wrench,
    Loader2,
    Clock,
    Check,
    X,
    Ban,
    ChevronRight,
    ChevronDown,
  } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    toolCall,
    expanded,
    clickable,
    elapsed,
    onToggle,
  }: {
    toolCall: ToolCall;
    expanded: boolean;
    clickable: boolean;
    elapsed: number;
    onToggle: () => void;
  } = $props();

  const TOOL_ICONS: Record<ToolCallType, typeof Search> = {
    web_search: Search,
    code_execution: Play,
    file_read: FileText,
    file_write: FilePen,
    api_call: Link,
    custom: Wrench,
  };

  const ToolIcon = $derived(TOOL_ICONS[toolCall.tool_type] ?? Wrench);
  const isPending = $derived(toolCall.status === "pending");
  const isInProgress = $derived(toolCall.status === "in_progress");
  const isCompleted = $derived(toolCall.status === "completed");
  const isFailed = $derived(toolCall.status === "failed");
  const isCancelled = $derived(toolCall.status === "cancelled");

  function formatElapsed(seconds: number): string {
    return `${seconds.toFixed(1)}s`;
  }
</script>

<button
  class="tool-call-header"
  class:clickable
  onclick={onToggle}
  disabled={!clickable}
  aria-expanded={clickable ? expanded : undefined}
>
  <span class="tool-icon">
    <ToolIcon size={ICON_SIZE.md} />
  </span>
  <span class="tool-name">{toolCall.tool_display_name}</span>
  {#if toolCall.started_at}
    <span class="tool-elapsed">{formatElapsed(elapsed)}</span>
  {/if}
  <span class="tool-status">
    {#if isPending}
      <Clock size={ICON_SIZE.sm} />
    {:else if isInProgress}
      <span class="spinner"><Loader2 size={ICON_SIZE.sm} /></span>
    {:else if isCompleted}
      <Check size={ICON_SIZE.sm} />
    {:else if isFailed}
      <X size={ICON_SIZE.sm} />
    {:else if isCancelled}
      <Ban size={ICON_SIZE.sm} />
    {/if}
  </span>
  {#if clickable}
    <span class="tool-chevron">
      {#if expanded}
        <ChevronDown size={ICON_SIZE.sm} />
      {:else}
        <ChevronRight size={ICON_SIZE.sm} />
      {/if}
    </span>
  {/if}
</button>

<style>
  .tool-call-header {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    padding: var(--space-3) var(--space-5);
    background: none;
    border: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-base);
    text-align: left;
  }

  .tool-call-header.clickable {
    cursor: pointer;
  }

  .tool-call-header.clickable:hover {
    background: var(--surface-overlay-faint);
  }

  .tool-call-header:disabled:not(.clickable) {
    cursor: default;
  }

  .tool-icon {
    display: flex;
    flex-shrink: 0;
    color: var(--text-muted);
  }

  .tool-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: var(--font-weight-medium);
  }

  .tool-status {
    display: flex;
    flex-shrink: 0;
  }

  .tool-elapsed {
    font-size: var(--font-size-sm);
    color: var(--text-disabled);
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
    min-width: 3ch;
    text-align: right;
  }

  .tool-chevron {
    display: flex;
    flex-shrink: 0;
    color: var(--text-disabled);
  }

  .spinner {
    display: flex;
    animation: spin 1s linear infinite;
  }

  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation: none;
    }
  }
</style>
