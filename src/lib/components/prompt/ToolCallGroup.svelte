<script lang="ts">
  import { slide } from "svelte/transition";
  import type { ToolCall } from "$lib/types/ai";
  import { ChevronRight, ChevronDown, Wrench } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import ToolCallItem from "./ToolCallItem.svelte";

  let {
    toolCalls,
    isStreaming = false,
    onApprove,
    onReject,
    onRetry,
  }: {
    toolCalls: ToolCall[];
    isStreaming?: boolean;
    onApprove?: (toolCallId: string) => void;
    onReject?: (toolCallId: string) => void;
    onRetry?: (toolCallId: string) => void;
  } = $props();

  let expanded = $state(true);
  let wasActive = $state(false);

  let allCompleted = $derived(
    toolCalls.length > 0 &&
    toolCalls.every((tc) => tc.status === "completed" || tc.status === "failed" || tc.status === "cancelled")
  );

  let anyActive = $derived(
    toolCalls.some((tc) => tc.status === "in_progress" || tc.status === "pending")
  );

  let failedCount = $derived(toolCalls.filter((tc) => tc.status === "failed").length);

  let summaryText = $derived.by(() => {
    const label = toolCalls.length === 1 ? "Used 1 tool" : `Used ${toolCalls.length} tools`;
    return failedCount > 0 ? `${label} (${failedCount} failed)` : label;
  });

  $effect(() => {
    if (anyActive) {
      wasActive = true;
    }
    if (allCompleted && wasActive && !isStreaming) {
      expanded = false;
    }
  });
</script>

{#if anyActive || (isStreaming && wasActive)}
  <div class="tool-group">
    <div class="tool-group-header-active">
      <Wrench size={ICON_SIZE.sm} />
      <span class="tool-group-label-active">Running tools</span>
    </div>
    <div class="tool-group-items">
      {#each toolCalls as toolCall (toolCall.tool_call_id)}
        <ToolCallItem {toolCall} {onApprove} {onReject} {onRetry} />
      {/each}
    </div>
  </div>
{:else if toolCalls.length > 0}
  <div class="tool-group">
    <button
      class="tool-group-toggle"
      onclick={() => (expanded = !expanded)}
      aria-expanded={expanded}
    >
      {#if expanded}
        <ChevronDown size={ICON_SIZE.sm} />
      {:else}
        <ChevronRight size={ICON_SIZE.sm} />
      {/if}
      <Wrench size={ICON_SIZE.sm} />
      <span class="tool-group-summary">{summaryText}</span>
    </button>
    {#if expanded}
      <div class="tool-group-items" transition:slide={{ duration: 150 }}>
        {#each toolCalls as toolCall (toolCall.tool_call_id)}
          <ToolCallItem {toolCall} {onApprove} {onReject} {onRetry} />
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .tool-group {
    margin: 4px 0;
  }

  .tool-group-header-active {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 0;
    color: rgba(255, 255, 255, 0.6);
  }

  .tool-group-label-active {
    font-size: 13px;
    font-weight: 600;
    background: linear-gradient(
      90deg,
      rgba(91, 141, 217, 0.6) 0%,
      rgba(150, 190, 240, 0.9) 50%,
      rgba(91, 141, 217, 0.6) 100%
    );
    background-size: 200% auto;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    animation: shimmer 2s linear infinite;
  }

  .tool-group-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 0;
    background: none;
    border: none;
    color: rgba(255, 255, 255, 0.45);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .tool-group-toggle:hover {
    color: rgba(255, 255, 255, 0.7);
  }

  .tool-group-summary {
    font-weight: 500;
  }

  .tool-group-items {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 4px;
  }

  @keyframes shimmer {
    0% { background-position: -200% center; }
    100% { background-position: 200% center; }
  }

  @media (prefers-reduced-motion: reduce) {
    .tool-group-label-active {
      animation: none;
      background: none;
      -webkit-text-fill-color: #96bef0;
    }
  }
</style>
