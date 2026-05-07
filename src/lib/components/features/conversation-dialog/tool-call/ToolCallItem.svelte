<script lang="ts">
  import type { ToolCall } from "$lib/types/ai";
  import ToolResultRenderer from "./ToolResultRenderer.svelte";
  import ToolCallHeader from "../components/ToolCallHeader.svelte";
  import ToolCallDetails from "../components/ToolCallDetails.svelte";
  import ToolCallApprovalActions from "../components/ToolCallApprovalActions.svelte";
  import { useElapsedTimer } from "../drivers/useElapsedTimer.svelte";

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

  const isPending = $derived(toolCall.status === "pending");
  const isInProgress = $derived(toolCall.status === "in_progress");
  const isCompleted = $derived(toolCall.status === "completed");
  const isFailed = $derived(toolCall.status === "failed");
  const isCancelled = $derived(toolCall.status === "cancelled");
  const isClickable = $derived(isCompleted || isFailed || isCancelled);

  const timer = useElapsedTimer({
    isActive: () => isInProgress,
    startedAt: () => toolCall.started_at,
    completedAt: () => toolCall.completed_at,
  });

  function toggleExpanded() {
    if (isClickable) expanded = !expanded;
  }
</script>

<div
  class="tool-call-item"
  class:clickable={isClickable}
  class:pending={isPending}
  class:in-progress={isInProgress}
  class:completed={isCompleted}
  class:failed={isFailed}
  class:cancelled={isCancelled}
>
  <ToolCallHeader
    {toolCall}
    {expanded}
    clickable={isClickable}
    elapsed={timer.elapsed}
    onToggle={toggleExpanded}
  />

  {#if isPending}
    <ToolCallApprovalActions
      onApprove={onApprove ? () => onApprove(toolCall.tool_call_id) : undefined}
      onReject={onReject ? () => onReject(toolCall.tool_call_id) : undefined}
    />
  {/if}

  {#if isInProgress && toolCall.result}
    <div class="in-progress-detail">
      <ToolResultRenderer result={toolCall.result} isPartial={true} />
    </div>
  {/if}

  {#if expanded && isClickable}
    <ToolCallDetails
      {toolCall}
      onRetry={onRetry ? () => onRetry(toolCall.tool_call_id) : undefined}
    />
  {/if}
</div>

<style>
  .tool-call-item {
    border-radius: var(--radius-md);
    overflow: hidden;
    font-size: var(--font-size-base);
    background: var(--surface-overlay-faint);
    transition: background var(--motion-fast) var(--ease-default);
  }

  .tool-call-item.clickable:hover {
    background: var(--surface-overlay);
  }

  .tool-call-item.pending {
    border-left: 3px solid var(--warning);
    background: var(--warning-bg-soft);
    animation: pulse 2s ease-in-out infinite;
  }

  .tool-call-item.in-progress {
    border-left: 3px solid var(--tool-running-stripe);
    background: var(--accent-bg-soft);
  }

  .tool-call-item.cancelled {
    background: transparent;
  }

  .tool-call-item.clickable.cancelled:hover {
    background: var(--surface-overlay-faint);
  }

  .tool-call-item.pending :global(.tool-status) {
    color: var(--warning);
  }

  .tool-call-item.in-progress :global(.tool-status) {
    color: var(--accent);
  }

  .tool-call-item.completed :global(.tool-status) {
    color: var(--success);
  }

  .tool-call-item.failed :global(.tool-status) {
    color: var(--danger);
  }

  .tool-call-item.cancelled :global(.tool-status) {
    color: var(--text-disabled);
  }

  .in-progress-detail {
    padding: var(--space-0) var(--space-5) var(--space-3);
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
  }
</style>
