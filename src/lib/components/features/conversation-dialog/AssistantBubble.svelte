<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { saveSvg } from "$lib/services/fileSave";
  import type { ConversationNode, ContentSegment } from "$lib/types/conversation";
  import type { ToolCall } from "$lib/types/ai";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";
  import MarkdownRenderer from "$lib/components/shared/ui/MarkdownRenderer.svelte";
  import ThinkingBlock from "$lib/components/shared/ui/ThinkingBlock.svelte";
  import ToolCallGroup from "./tool-call/ToolCallGroup.svelte";
  import ProcessingIndicator from "./components/ProcessingIndicator.svelte";
  import ErrorBanner from "./components/ErrorBanner.svelte";
  import BranchNav from "./components/BranchNav.svelte";
  import ToolCallReadOnlyChip from "./components/ToolCallReadOnlyChip.svelte";
  import BubbleEditField from "./components/BubbleEditField.svelte";
  import BubbleActionsFooter from "./components/BubbleActionsFooter.svelte";
  import { useEditSegments } from "./drivers/useEditSegments.svelte";
  import { Copy, Check, RefreshCw, Trash2, Pencil, Save } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    node,
    displayContent,
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

  const TOOL_CALL_MARKER_PATTERN = /\{\{tool_call:([^}]+)\}\}/g;
  const TOOL_CALL_MARKER_TEST = /\{\{tool_call:[^}]+\}\}/;

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

  let hasToolCalls = $derived(node.tool_calls.length > 0 || activeToolCalls.length > 0);

  let allToolCallsDone = $derived(
    hasToolCalls &&
    activeToolCalls.length === 0 &&
    node.tool_calls.length > 0 &&
    node.tool_calls.every((tc) => tc.status === "completed" || tc.status === "failed" || tc.status === "cancelled")
  );

  let isProcessingToolResults = $derived(
    isStreaming && !isThinkingActive && allToolCallsDone
  );

  let isWaitingForContent = $derived(
    isStreaming && !isThinkingActive && !thinkingContent && !displayContent && activeToolCalls.length === 0
  );

  let showGenerating = $derived(isProcessingToolResults || isWaitingForContent);

  let allToolCalls = $derived.by(() => {
    const map = new Map<string, ToolCall>();
    for (const tc of node.tool_calls) map.set(tc.tool_call_id, tc);
    for (const tc of activeToolCalls) map.set(tc.tool_call_id, tc);
    return map;
  });

  type RenderBlock =
    | { kind: "text"; text: string }
    | { kind: "tool_group"; toolCallIds: string[] };

  let renderBlocks = $derived.by(() => {
    const blocks: RenderBlock[] = [];
    for (const seg of segments) {
      if (seg.type === "text" && seg.text.trim()) {
        blocks.push({ kind: "text", text: seg.text });
      } else if (seg.type === "tool_call") {
        const last = blocks[blocks.length - 1];
        if (last && last.kind === "tool_group") {
          last.toolCallIds.push(seg.tool_call_id);
        } else {
          blocks.push({ kind: "tool_group", toolCallIds: [seg.tool_call_id] });
        }
      }
    }
    return blocks;
  });

  let editMode = $state(false);
  const edit = useEditSegments({ parseContent: parseContentSegments });

  function toggleEditMode() {
    if (!editMode) {
      edit.enter(displayContent);
      editMode = true;
    } else {
      edit.exit();
      editMode = false;
    }
  }

  function submitEdit() {
    onContentChange(edit.rebuild());
    edit.exit();
    editMode = false;
  }

  function handleEditKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      submitEdit();
    }
  }

  async function copyContent() {
    await navigator.clipboard.writeText(displayContent);
  }

  function formatDuration(seconds: number): string {
    if (seconds < 60) return `${seconds.toFixed(1)}s`;
    const m = Math.floor(seconds / 60);
    const s = Math.round(seconds % 60);
    return s > 0 ? `${m}m ${s}s` : `${m}m`;
  }
</script>

<div class="assistant-message-wrapper">
  <div class="assistant-bubble" class:editing={editMode}>
    <div class="bubble-body">
      {#if thinkingContent || isThinkingActive}
        <ThinkingBlock {thinkingContent} {isThinkingActive} {isStreaming} thinkingDuration={node.thinking_duration} />
      {/if}

      {#if editMode}
        <BubbleEditField variant="assistant">
          {#each edit.segments as seg, i (i)}
            {#if seg.type === "text" && seg.text.trim() !== ""}
              <textarea
                bind:this={edit.textareaRefs[i]}
                value={seg.text}
                oninput={(e) => edit.onSegmentInput(i, e)}
                onkeydown={handleEditKeydown}
                class="bubble-textarea"
                rows="1"
              ></textarea>
            {:else if seg.type === "tool_call"}
              {@const tc = allToolCalls.get(seg.tool_call_id)}
              <ToolCallReadOnlyChip label={tc?.tool_name ?? "tool call"} />
            {/if}
          {/each}
        </BubbleEditField>
      {:else if hasMarkers}
        {#each renderBlocks as block}
          {#if block.kind === "text"}
            <MarkdownRenderer content={block.text} {isStreaming} onopen={openUrl} onsavesvg={saveSvg} />
          {:else if block.kind === "tool_group"}
            {@const groupToolCalls = block.toolCallIds
              .map((id) => allToolCalls.get(id))
              .filter((tc): tc is ToolCall => tc != null)}
            {#if groupToolCalls.length > 0}
              <ToolCallGroup
                toolCalls={groupToolCalls}
                {isStreaming}
                onApprove={onToolCallApprove}
                onReject={onToolCallReject}
                onRetry={onToolCallRetry}
              />
            {/if}
          {/if}
        {/each}
      {:else if displayContent}
        <MarkdownRenderer content={displayContent} {isStreaming} onopen={openUrl} onsavesvg={saveSvg} />
      {/if}

      {#if showGenerating}
        <ProcessingIndicator />
      {/if}

      {#if node.error}
        <ErrorBanner message={node.error} onRetry={() => onRegenerate(node.node_id)} />
      {:else if node.cancelled}
        <ErrorBanner message="Response interrupted" variant="cancelled" />
      {/if}
    </div>
  </div>

  <BubbleActionsFooter visible={editMode}>
    {#if node.query_duration != null}
      <span class="query-time" title="Query time">{formatDuration(node.query_duration)}</span>
    {/if}
    <div class="bubble-actions">
      {#if branchInfo.total > 1}
        <BranchNav
          current={branchInfo.current}
          total={branchInfo.total}
          onPrev={() => onBranchPrev(node.node_id)}
          onNext={() => onBranchNext(node.node_id)}
        />
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
      {#if editMode}
        <ActionIconButton
          icon={Save}
          onclick={submitEdit}
          title="Save (Ctrl+Enter)"
        />
      {/if}

      {#if showDelete}
        <button class="icon-btn delete-btn" onclick={() => onDelete(node.node_id)} title="Delete">
          <Trash2 size={ICON_SIZE.md} />
        </button>
      {/if}
    </div>
  </BubbleActionsFooter>
</div>

<style>
  .assistant-bubble {
    padding: var(--space-7) var(--space-10);
    border-radius: var(--radius-lg);
    user-select: none;
    -webkit-user-select: none;
  }

  .query-time {
    font-size: var(--font-size-sm);
    color: var(--text-disabled);
    font-weight: var(--font-weight-regular);
  }

  .bubble-actions {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-2);
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .icon-btn:hover {
    background: var(--surface-overlay);
    color: var(--text-secondary);
  }

  .icon-btn.active {
    background: var(--surface-overlay-strong);
    color: var(--text-primary);
  }

  .delete-btn:hover {
    background: var(--danger-border);
    border-color: var(--danger-border);
    color: var(--danger);
  }

  .bubble-textarea {
    width: 100%;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--font-size-lg);
    line-height: var(--line-height-normal);
    padding: var(--space-2) var(--space-0) var(--space-4);
    resize: none;
    overflow: hidden;
    box-sizing: border-box;
  }

  .bubble-textarea:focus {
    outline: none;
  }
</style>
