<script lang="ts">
  import type { ToolCall } from "$lib/types/ai";
  import ToolCallItem from "./ToolCallItem.svelte";

  let {
    toolCalls,
    onApprove,
    onReject,
    onRetry,
  }: {
    toolCalls: ToolCall[];
    onApprove?: (toolCallId: string) => void;
    onReject?: (toolCallId: string) => void;
    onRetry?: (toolCallId: string) => void;
  } = $props();
</script>

<div class="tool-call-group">
  {#each toolCalls as toolCall (toolCall.tool_call_id)}
    <ToolCallItem {toolCall} {onApprove} {onReject} {onRetry} />
  {/each}
</div>

<style>
  .tool-call-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
</style>
