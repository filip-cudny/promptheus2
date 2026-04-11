<script lang="ts">
  import { onDestroy } from "svelte";
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
  let elapsed = $state(0);
  let intervalId: ReturnType<typeof setInterval> | null = null;

  const TOOL_ICONS: Record<ToolCallType, typeof Search> = {
    builtin_web_search: Search,
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
  let hasExpandableContent = $derived(
    (toolCall.arguments && Object.keys(toolCall.arguments).length > 0) ||
    !!toolCall.result ||
    !!toolCall.error
  );
  let isClickable = $derived((isCompleted || isFailed || isCancelled) && hasExpandableContent);

  $effect(() => {
    if (isInProgress && toolCall.started_at) {
      const start = new Date(toolCall.started_at).getTime();
      elapsed = (Date.now() - start) / 1000;
      intervalId = setInterval(() => {
        elapsed = (Date.now() - start) / 1000;
      }, 100);
      return () => {
        if (intervalId) clearInterval(intervalId);
      };
    } else if (toolCall.started_at && toolCall.completed_at) {
      elapsed =
        (new Date(toolCall.completed_at).getTime() -
          new Date(toolCall.started_at).getTime()) /
        1000;
    }
  });

  onDestroy(() => {
    if (intervalId) clearInterval(intervalId);
  });

  function formatElapsed(seconds: number): string {
    if (seconds < 10) return `${seconds.toFixed(1)}s`;
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
    {#if toolCall.started_at}
      <span class="tool-elapsed">{formatElapsed(elapsed)}</span>
    {/if}
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
      <span class="partial-result">{toolCall.result}</span>
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
          <span class="details-label">{toolCall.tool_type === "builtin_web_search" ? "Queries" : "Output"}</span>
          <div class="details-content result-scroll">{toolCall.result}</div>
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
    border-radius: 4px;
    overflow: hidden;
    font-size: 13px;
  }

  .tool-call-item.pending {
    border-left: 3px solid #d4a843;
    background: rgba(212, 168, 67, 0.06);
    animation: pulse 2s ease-in-out infinite;
  }

  .tool-call-item.in-progress {
    border-left: 3px solid #5b8dd9;
    background: linear-gradient(
      90deg,
      rgba(91, 141, 217, 0.06) 0%,
      rgba(91, 141, 217, 0.12) 50%,
      rgba(91, 141, 217, 0.06) 100%
    );
    background-size: 200% auto;
    animation: shimmer 2s linear infinite;
  }

  .tool-call-item.completed {
    border-left: 3px solid rgba(92, 184, 92, 0.4);
    background: rgba(92, 184, 92, 0.04);
  }

  .tool-call-item.failed {
    border-left: 3px solid rgba(238, 85, 85, 0.4);
    background: rgba(220, 60, 60, 0.06);
  }

  .tool-call-item.cancelled {
    border-left: 3px solid rgba(255, 255, 255, 0.15);
    background: transparent;
  }

  .tool-call-header {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    background: none;
    border: none;
    color: rgba(255, 255, 255, 0.8);
    font: inherit;
    font-size: 13px;
    text-align: left;
  }

  .tool-call-header.clickable {
    cursor: pointer;
  }

  .tool-call-header.clickable:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .tool-call-header:disabled:not(.clickable) {
    cursor: default;
  }

  .tool-icon {
    display: flex;
    flex-shrink: 0;
    color: rgba(255, 255, 255, 0.6);
  }

  .tool-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }

  .tool-status {
    display: flex;
    flex-shrink: 0;
  }

  .pending .tool-status {
    color: #d4a843;
  }

  .in-progress .tool-status {
    color: #5b8dd9;
  }

  .completed .tool-status {
    color: #5cb85c;
  }

  .failed .tool-status {
    color: #e55;
  }

  .cancelled .tool-status {
    color: rgba(255, 255, 255, 0.3);
  }

  .spinner {
    display: flex;
    animation: spin 1s linear infinite;
  }

  .tool-elapsed {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.35);
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
    min-width: 3ch;
    text-align: right;
  }

  .tool-chevron {
    display: flex;
    flex-shrink: 0;
    color: rgba(255, 255, 255, 0.35);
  }

  .confirmation-actions {
    display: flex;
    gap: 8px;
    padding: 4px 10px 8px;
  }

  .approve-btn,
  .reject-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    border: none;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .approve-btn {
    background: rgba(92, 184, 92, 0.15);
    color: #5cb85c;
  }

  .approve-btn:hover {
    background: rgba(92, 184, 92, 0.25);
  }

  .reject-btn {
    background: rgba(220, 60, 60, 0.1);
    color: rgba(255, 255, 255, 0.5);
  }

  .reject-btn:hover {
    background: rgba(220, 60, 60, 0.2);
    color: #e55;
  }

  .in-progress-detail {
    padding: 0 10px 6px;
  }

  .partial-result {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.4);
  }

  .tool-call-details {
    max-height: 400px;
    overflow: hidden;
  }

  .details-separator {
    height: 1px;
    background: rgba(255, 255, 255, 0.08);
    margin: 0 10px;
  }

  .details-section {
    padding: 6px 10px;
  }

  .details-label {
    display: block;
    font-size: 11px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.4);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 4px;
  }

  .details-content {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.65);
    line-height: 1.5;
  }

  .argument-line {
    padding: 1px 0;
    word-break: break-word;
  }

  .result-scroll {
    max-height: 300px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .error-text {
    color: #e55;
  }

  .retry-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    margin: 4px 10px 8px;
    padding: 4px 8px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }

  .retry-btn:hover {
    background: rgba(220, 60, 60, 0.15);
    color: #e55;
  }

  @keyframes shimmer {
    0% {
      background-position: -200% center;
    }
    100% {
      background-position: 200% center;
    }
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
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
      background: rgba(212, 168, 67, 0.06);
    }

    .tool-call-item.in-progress {
      animation: none;
      background: rgba(91, 141, 217, 0.08);
    }

    .spinner {
      animation: none;
    }
  }
</style>
