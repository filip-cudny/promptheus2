<script lang="ts">
  import { slide } from "svelte/transition";
  import { RefreshCw } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import type { ToolCall } from "$lib/types/ai";
  import ToolResultRenderer from "../tool-call/ToolResultRenderer.svelte";

  let {
    toolCall,
    onRetry,
  }: {
    toolCall: ToolCall;
    onRetry?: () => void;
  } = $props();

  const isFailed = $derived(toolCall.status === "failed");
  const hasArguments = $derived(
    !!toolCall.arguments && Object.keys(toolCall.arguments).length > 0,
  );

  function formatArguments(args: Record<string, unknown>): string[] {
    return Object.entries(args).map(
      ([key, value]) =>
        `${key}: ${typeof value === "string" ? value : JSON.stringify(value)}`,
    );
  }
</script>

<div class="tool-call-details" transition:slide={{ duration: 200 }}>
  <div class="details-separator"></div>
  {#if hasArguments}
    <div class="details-section">
      <span class="details-label">Input</span>
      <div class="details-content">
        {#each formatArguments(toolCall.arguments!) as line}
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
    <button class="retry-btn" onclick={onRetry}>
      <RefreshCw size={ICON_SIZE.sm} />
      Retry
    </button>
  {/if}
</div>

<style>
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
</style>
