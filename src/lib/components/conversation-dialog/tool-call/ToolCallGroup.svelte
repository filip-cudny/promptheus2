<script lang="ts">
  import { slide } from "svelte/transition";
  import type { ToolCall } from "$lib/types/ai";
  import { ChevronRight, ChevronDown, Wrench } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import ToolCallItem from "./ToolCallItem.svelte";
  import ProcessingIndicator from "../components/ProcessingIndicator.svelte";

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

  let allCompletedOnMount = toolCalls.length > 0 &&
    toolCalls.every((tc) => tc.status === "completed" || tc.status === "failed" || tc.status === "cancelled");

  let expanded = $state(!allCompletedOnMount);
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
      <ProcessingIndicator label="Running tools" inline />
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
    margin: var(--space-2) var(--space-0);
    border-left: 3px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: rgba(255, 255, 255, 0.03);
    overflow: hidden;
  }

  .tool-group-header-active {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    padding: var(--space-3) var(--space-5);
    color: var(--text-secondary);
    font-size: var(--font-size-base);
  }

  .tool-group-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    padding: var(--space-3) var(--space-5);
    border: none;
    background: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-base);
    text-align: left;
    cursor: pointer;
  }

  .tool-group-toggle:hover {
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
  }

  .tool-group-summary {
    font-weight: var(--font-weight-medium);
  }

  .tool-group-items {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4) var(--space-5) var(--space-4);
  }
</style>
