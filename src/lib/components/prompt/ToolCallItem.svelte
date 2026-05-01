<script lang="ts">
  import { slide } from "svelte/transition";
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
    RefreshCw,
  } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import ToolResultRenderer from "./ToolResultRenderer.svelte";
  import { useElapsedTimer } from "./drivers/useElapsedTimer.svelte";

  let {
    toolCall,
    onApprove,
    onReject,
    onRetry,
  }: {
    toolCall: ToolCall;
    onApprove?: (toolCallId: string) => void;
    onReject?: (toolCallId: string) => void;
    onRetry?: (toolCallId: string) => void;
  } = $props();

  let expanded = $state(false);

  const TOOL_ICONS: Record<ToolCallType, typeof Search> = {
    web_search: Search,
    code_execution: Play,
    file_read: FileText,
    file_write: FilePen,
    api_call: Link,
    custom: Wrench,
  };

  let ToolIcon = $derived(TOOL_ICONS[toolCall.tool_type] ?? Wrench);
  let isInProgress = $derived(toolCall.status === "in_progress");
  let isPending = $derived(toolCall.status === "pending");
  let isCompleted = $derived(toolCall.status === "completed");
  let isFailed = $derived(toolCall.status === "failed");
  let isCancelled = $derived(toolCall.status === "cancelled");
  let isClickable = $derived(isCompleted || isFailed || isCancelled);

  const timer = useElapsedTimer({
    isActive: () => isInProgress,
    startedAt: () => toolCall.started_at,
    completedAt: () => toolCall.completed_at,
  });

  function formatElapsed(seconds: number): string {
    return `${seconds.toFixed(1)}s`;
  }

  function formatArguments(args: Record<string, unknown>): string[] {
    return Object.entries(args).map(
      ([key, value]) =>
        `${key}: ${typeof value === "string" ? value : JSON.stringify(value)}`,
    );
  }

  function toggleExpanded() {
    if (isClickable) expanded = !expanded;
  }
</script>

<div
  class="tool-call-item"
  class:pending={isPending}
  class:in-progress={isInProgress}
  class:completed={isCompleted}
  class:failed={isFailed}
  class:cancelled={isCancelled}
>
  <button
    class="tool-call-header"
    class:clickable={isClickable}
    onclick={toggleExpanded}
    disabled={!isClickable}
    aria-expanded={isClickable ? expanded : undefined}
  >
    <span class="tool-icon">
      <ToolIcon size={ICON_SIZE.md} />
    </span>
    <span class="tool-name">{toolCall.tool_display_name}</span>
    {#if toolCall.started_at}
      <span class="tool-elapsed">{formatElapsed(timer.elapsed)}</span>
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
    {#if isClickable}
      <span class="tool-chevron">
        {#if expanded}
          <ChevronDown size={ICON_SIZE.sm} />
        {:else}
          <ChevronRight size={ICON_SIZE.sm} />
        {/if}
      </span>
    {/if}
  </button>

  {#if isPending && (onApprove || onReject)}
    <div class="confirmation-actions">
      {#if onApprove}
        <button class="approve-btn" onclick={() => onApprove(toolCall.tool_call_id)}>
          <Check size={ICON_SIZE.sm} />
          Approve
        </button>
      {/if}
      {#if onReject}
        <button class="reject-btn" onclick={() => onReject(toolCall.tool_call_id)}>
          <X size={ICON_SIZE.sm} />
          Reject
        </button>
      {/if}
    </div>
  {/if}

  {#if isInProgress && toolCall.result}
    <div class="in-progress-detail">
      <ToolResultRenderer result={toolCall.result} isPartial={true} />
    </div>
  {/if}

  {#if expanded && isClickable}
    <div class="tool-call-details" transition:slide={{ duration: 200 }}>
      <div class="details-separator"></div>
      {#if toolCall.arguments && Object.keys(toolCall.arguments).length > 0}
        <div class="details-section">
          <span class="details-label">Input</span>
          <div class="details-content">
            {#each formatArguments(toolCall.arguments) as line}
              <div class="argument-line">{line}</div>
            {/each}
          </div>
        </div>
      {/if}
      {#if toolCall.result}
        <div class="details-section">
          <span class="details-label">{toolCall.tool_type === "web_search" ? "Queries" : "Output"}</span>
          <div class="details-content result-scroll">
            <ToolResultRenderer result={toolCall.result} />
          </div>
        </div>
      {/if}
      {#if toolCall.error}
        <div class="details-section">
          <span class="details-label">Error</span>
          <div class="details-content error-text">{toolCall.error}</div>
        </div>
      {/if}
      {#if isFailed && onRetry}
        <button class="retry-btn" onclick={() => onRetry(toolCall.tool_call_id)}>
          <RefreshCw size={ICON_SIZE.sm} />
          Retry
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .tool-call-item {
    border-radius: var(--radius-md);
    overflow: hidden;
    font-size: var(--font-size-base);
  }

  .tool-call-item.pending {
    border-left: 3px solid var(--warning);
    background: var(--warning-bg-soft);
    animation: pulse 2s ease-in-out infinite;
  }

  .tool-call-item.in-progress {
    border-left: 3px solid var(--accent);
    background: linear-gradient(
      90deg,
      var(--accent-bg-soft) 0%,
      var(--accent-bg-soft) 50%,
      var(--accent-bg-soft) 100%
    );
    background-size: 200% auto;
    animation: shimmer 2s linear infinite;
  }

  .tool-call-item.completed {
    border-left: 3px solid var(--success-border);
    background: var(--success-bg-soft);
  }

  .tool-call-item.failed {
    border-left: 3px solid var(--danger-border);
    background: var(--danger-bg-soft);
  }

  .tool-call-item.cancelled {
    border-left: 3px solid var(--border-strong);
    background: transparent;
  }

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

  .pending .tool-status {
    color: var(--warning);
  }

  .in-progress .tool-status {
    color: var(--accent);
  }

  .completed .tool-status {
    color: var(--success);
  }

  .failed .tool-status {
    color: var(--danger);
  }

  .cancelled .tool-status {
    color: var(--text-disabled);
  }

  .spinner {
    display: flex;
    animation: spin 1s linear infinite;
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

  .confirmation-actions {
    display: flex;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-5) var(--space-4);
  }

  .approve-btn,
  .reject-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-5);
    border: none;
    border-radius: var(--radius-md);
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
  }

  .approve-btn {
    background: var(--success-bg-soft);
    color: var(--success);
  }

  .approve-btn:hover {
    background: var(--success-border);
  }

  .reject-btn {
    background: var(--danger-bg-soft);
    color: var(--text-muted);
  }

  .reject-btn:hover {
    background: var(--danger-border);
    color: var(--danger);
  }

  .in-progress-detail {
    padding: var(--space-0) var(--space-5) var(--space-3);
  }

  .tool-call-details {
    max-height: 400px;
    overflow: hidden;
  }

  .details-separator {
    height: 1px;
    background: var(--surface-overlay);
    margin: var(--space-0) var(--space-5);
  }

  .details-section {
    padding: var(--space-3) var(--space-5);
  }

  .details-label {
    display: block;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    color: var(--text-disabled);
    text-transform: uppercase;
    letter-spacing: var(--tracking-label);
    margin-bottom: var(--space-2);
  }

  .details-content {
    font-size: var(--font-size-md);
    color: var(--text-muted);
    line-height: var(--line-height-normal);
  }

  .argument-line {
    padding: 1px var(--space-0);
    word-break: break-word;
  }

  .result-scroll {
    max-height: 300px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .error-text {
    color: var(--danger);
  }

  .retry-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: var(--space-2) var(--space-5) var(--space-4);
    padding: var(--space-2) var(--space-4);
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
  }

  .retry-btn:hover {
    background: var(--danger-bg-soft);
    color: var(--danger);
  }

  @keyframes pulse {
    0%,
    100% {
      background-color: rgba(212, 168, 67, 0.04);
    }
    50% {
      background-color: rgba(212, 168, 67, 0.1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tool-call-item.pending {
      animation: none;
      background: var(--warning-bg-soft);
    }

    .tool-call-item.in-progress {
      animation: none;
      background: var(--accent-bg-soft);
    }

    .spinner {
      animation: none;
    }
  }
</style>
