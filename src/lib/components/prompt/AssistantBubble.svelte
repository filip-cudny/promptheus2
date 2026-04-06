<script lang="ts">
  import type { ConversationNode, ContentSegment } from "$lib/types/conversation";
  import type { ToolCall } from "$lib/types/ai";
  import CollapsibleSection from "$lib/components/ui/CollapsibleSection.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import MarkdownRenderer from "$lib/components/ui/MarkdownRenderer.svelte";
  import ThinkingBlock from "$lib/components/ui/ThinkingBlock.svelte";
  import ToolCallItem from "./ToolCallItem.svelte";
  import { resizeTextarea } from "$lib/utils/autoResize";
  import { Copy, Check, RefreshCw, Trash2, ChevronLeft, ChevronRight, Pencil, AlertCircle } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    node,
    displayContent,
    outputNumber,
    showDelete = false,
    isStreaming = false,
    thinkingContent = "",
    isThinkingActive = false,
    branchInfo = { current: 1, total: 1 },
    activeToolCalls = [],
    onRegenerate,
    onBranchPrev,
    onBranchNext,
    onContentChange,
    onDelete,
    onToolCallApprove,
    onToolCallReject,
    onToolCallRetry,
  }: {
    node: ConversationNode;
    displayContent: string;
    outputNumber: number;
    showDelete: boolean;
    isStreaming: boolean;
    thinkingContent: string;
    isThinkingActive: boolean;
    branchInfo: { current: number; total: number };
    activeToolCalls: ToolCall[];
    onRegenerate: (nodeId: string) => void;
    onBranchPrev: (nodeId: string) => void;
    onBranchNext: (nodeId: string) => void;
    onContentChange: (content: string) => void;
    onDelete: (nodeId: string) => void;
    onToolCallApprove?: (toolCallId: string) => void;
    onToolCallReject?: (toolCallId: string) => void;
    onToolCallRetry?: (toolCallId: string) => void;
  } = $props();

  const TOOL_CALL_MARKER_PATTERN = /\{\{tool_call:([a-zA-Z0-9_-]+)\}\}/g;
  const TOOL_CALL_MARKER_TEST = /\{\{tool_call:[a-zA-Z0-9_-]+\}\}/;

  function parseContentSegments(content: string): ContentSegment[] {
    const segments: ContentSegment[] = [];
    let lastIndex = 0;

    for (const match of content.matchAll(TOOL_CALL_MARKER_PATTERN)) {
      if (match.index > lastIndex) {
        segments.push({ type: "text", text: content.slice(lastIndex, match.index) });
      }
      segments.push({ type: "tool_call", tool_call_id: match[1] });
      lastIndex = match.index + match[0].length;
    }

    if (lastIndex < content.length) {
      segments.push({ type: "text", text: content.slice(lastIndex) });
    }

    return segments;
  }

  let hasMarkers = $derived(TOOL_CALL_MARKER_TEST.test(displayContent));

  let segments = $derived(hasMarkers ? parseContentSegments(displayContent) : []);

  let allToolCalls = $derived.by(() => {
    const map = new Map<string, ToolCall>();
    for (const tc of node.tool_calls) map.set(tc.tool_call_id, tc);
    for (const tc of activeToolCalls) map.set(tc.tool_call_id, tc);
    return map;
  });

  let collapsed = $state(false);
  let editMode = $state(false);
  let textarea: HTMLTextAreaElement | undefined = $state();

  $effect(() => {
    if (editMode && textarea) {
      displayContent;
      requestAnimationFrame(() => resizeTextarea(textarea!));
    }
  });

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    onContentChange(target.value);
    resizeTextarea(target);
  }

  function toggleEditMode() {
    editMode = !editMode;
    if (editMode && textarea) {
      requestAnimationFrame(() => resizeTextarea(textarea!));
    }
  }

  async function copyContent() {
    await navigator.clipboard.writeText(displayContent);
  }
</script>

<div class="assistant-bubble">
  <CollapsibleSection title="" bind:collapsed hoverActions actionsVisible={editMode}>
    {#snippet headerLeft()}
      <span class="role-badge assistant-badge">Assistant</span>
      <span class="turn-number"># {outputNumber}</span>
    {/snippet}
    {#snippet actions()}
      {#if branchInfo.total > 1}
        <span class="branch-nav">
          <button
            class="branch-btn"
            onclick={() => onBranchPrev(node.node_id)}
            disabled={branchInfo.current <= 1}
          ><ChevronLeft size={ICON_SIZE.md} /></button>
          <span class="branch-counter">{branchInfo.current}/{branchInfo.total}</span>
          <button
            class="branch-btn"
            onclick={() => onBranchNext(node.node_id)}
            disabled={branchInfo.current >= branchInfo.total}
          ><ChevronRight size={ICON_SIZE.md} /></button>
        </span>
      {/if}

      <ActionIconButton
        icon={RefreshCw}
        onclick={() => onRegenerate(node.node_id)}
        title="Regenerate"
      />
      <ActionIconButton
        icon={Copy}
        confirmIcon={Check}
        onclick={copyContent}
        title="Copy"
      />
      <button class="icon-btn" class:active={editMode} onclick={toggleEditMode} title={editMode ? "View" : "Edit"}>
        <Pencil size={ICON_SIZE.md} />
      </button>

      {#if showDelete}
        <button class="icon-btn delete-btn" onclick={() => onDelete(node.node_id)} title="Delete">
          <Trash2 size={ICON_SIZE.md} />
        </button>
      {/if}
    {/snippet}

    {#if thinkingContent || isThinkingActive}
      <ThinkingBlock {thinkingContent} {isThinkingActive} {isStreaming} />
    {/if}

    {#if editMode}
      <div class="bubble-edit-field">
        <textarea
          bind:this={textarea}
          value={displayContent}
          oninput={handleInput}
          class="bubble-textarea"
          rows="1"
        ></textarea>
      </div>
    {:else if hasMarkers}
      {#each segments as segment}
        {#if segment.type === "text" && segment.text.trim()}
          <MarkdownRenderer content={segment.text} {isStreaming} />
        {:else if segment.type === "tool_call"}
          {@const tc = allToolCalls.get(segment.tool_call_id)}
          {#if tc}
            <ToolCallItem
              toolCall={tc}
              onApprove={onToolCallApprove}
              onReject={onToolCallReject}
              onRetry={onToolCallRetry}
            />
          {/if}
        {/if}
      {/each}
    {:else if displayContent}
      <MarkdownRenderer content={displayContent} {isStreaming} />
    {/if}

    {#if node.error}
      <div class="error-banner">
        <AlertCircle size={ICON_SIZE.sm} />
        <span class="error-text">{node.error}</span>
        <button class="retry-btn" onclick={() => onRegenerate(node.node_id)}>
          <RefreshCw size={ICON_SIZE.sm} />
          Retry
        </button>
      </div>
    {:else if node.cancelled}
      <span class="cancelled-hint">Response interrupted</span>
    {/if}
  </CollapsibleSection>
</div>

<style>
  .assistant-bubble {
    border-left: 3.5px solid #9b6dcc;
    border-radius: 6px;
  }

  .role-badge {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .assistant-badge {
    background: rgba(155, 109, 204, 0.25);
    color: #c9a5f0;
  }

  .turn-number {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.4);
    font-weight: 500;
  }

  .branch-nav {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }

  .branch-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.5);
    border-radius: 4px;
    padding: 2px;
    cursor: pointer;
  }

  .branch-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.8);
  }

  .branch-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .branch-counter {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    min-width: 24px;
    text-align: center;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
  }

  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.8);
  }

  .icon-btn.active {
    background: rgba(155, 109, 204, 0.2);
    color: #c9a5f0;
  }

  .delete-btn:hover {
    background: rgba(200, 60, 60, 0.3);
    border-color: rgba(200, 60, 60, 0.5);
    color: #ff8a8a;
  }

  .bubble-edit-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    padding: 8px 8px 0;
  }

  .bubble-edit-field:focus-within {
    border-color: rgba(155, 109, 204, 0.4);
  }

  .bubble-textarea {
    width: 100%;
    background: transparent;
    border: none;
    color: #e0e0e0;
    font-family: "Fira Code", "Cascadia Code", monospace;
    font-size: 14px;
    line-height: 1.5;
    padding: 4px 0 8px;
    resize: none;
    overflow: hidden;
    box-sizing: border-box;
  }

  .bubble-textarea:focus {
    outline: none;
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
    padding: 8px 10px;
    background: rgba(220, 60, 60, 0.08);
    border-radius: 4px;
    color: #e55;
    font-size: 12px;
  }

  .error-banner :global(svg:first-child) {
    flex-shrink: 0;
  }

  .error-text {
    flex: 1;
    min-width: 0;
    overflow-wrap: break-word;
  }

  .retry-btn {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 4px;
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

  .cancelled-hint {
    display: block;
    margin-top: 6px;
    font-size: 11px;
    font-style: italic;
    color: rgba(255, 255, 255, 0.3);
  }
</style>
