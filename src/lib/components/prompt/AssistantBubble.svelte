<script lang="ts">
  import type { ConversationNode, ContentSegment } from "$lib/types/conversation";
  import type { ToolCall } from "$lib/types/ai";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import MarkdownRenderer from "$lib/components/ui/MarkdownRenderer.svelte";
  import ThinkingBlock from "$lib/components/ui/ThinkingBlock.svelte";
  import ToolCallGroup from "./ToolCallGroup.svelte";
  import { resizeTextarea } from "$lib/utils/autoResize";
  import { Copy, Check, RefreshCw, Trash2, ChevronLeft, ChevronRight, Pencil, AlertCircle, Wrench, Save } from "lucide-svelte";
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

  type EditSegment =
    | { type: "text"; leadingWs: string; text: string; trailingWs: string }
    | { type: "tool_call"; tool_call_id: string };

  let editMode = $state(false);
  let editSegments = $state<EditSegment[]>([]);
  let textareaRefs: Array<HTMLTextAreaElement | undefined> = $state([]);

  function splitTextSegment(text: string): { leadingWs: string; text: string; trailingWs: string } {
    const leadingWs = text.match(/^\s*/)?.[0] ?? "";
    const rest = text.slice(leadingWs.length);
    const trailingWs = rest.match(/\s*$/)?.[0] ?? "";
    const mid = rest.slice(0, rest.length - trailingWs.length);
    return { leadingWs, text: mid, trailingWs };
  }

  function buildEditSegments(content: string): EditSegment[] {
    const parsed = parseContentSegments(content);
    if (parsed.length === 0) {
      const split = splitTextSegment(content);
      return [{ type: "text", ...split }];
    }
    return parsed.map((s) =>
      s.type === "text"
        ? { type: "text" as const, ...splitTextSegment(s.text) }
        : { type: "tool_call" as const, tool_call_id: s.tool_call_id }
    );
  }

  function rebuildContentFromEditSegments(): string {
    return editSegments
      .map((s) =>
        s.type === "text"
          ? s.leadingWs + s.text + s.trailingWs
          : `{{tool_call:${s.tool_call_id}}}`
      )
      .join("");
  }

  function handleSegmentInput(idx: number, e: Event) {
    const target = e.target as HTMLTextAreaElement;
    const seg = editSegments[idx];
    if (seg.type !== "text") return;
    editSegments[idx] = { ...seg, text: target.value };
    resizeTextarea(target);
  }

  function toggleEditMode() {
    if (!editMode) {
      editSegments = buildEditSegments(displayContent);
    }
    editMode = !editMode;
    if (editMode) {
      requestAnimationFrame(() => {
        for (const ta of textareaRefs) {
          if (ta) resizeTextarea(ta);
        }
      });
    }
  }

  function submitEdit() {
    onContentChange(rebuildContentFromEditSegments());
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
        <div class="bubble-edit-field">
          {#each editSegments as seg, i (i)}
            {#if seg.type === "text" && seg.text.trim() !== ""}
              <textarea
                bind:this={textareaRefs[i]}
                value={seg.text}
                oninput={(e) => handleSegmentInput(i, e)}
                onkeydown={handleEditKeydown}
                class="bubble-textarea"
                rows="1"
              ></textarea>
            {:else if seg.type === "tool_call"}
              {@const tc = allToolCalls.get(seg.tool_call_id)}
              <div class="tool-call-chip" title="Tool call (read-only)">
                <Wrench size={ICON_SIZE.sm} />
                <span>{tc?.tool_name ?? "tool call"}</span>
              </div>
            {/if}
          {/each}
        </div>
      {:else if hasMarkers}
        {#each renderBlocks as block}
          {#if block.kind === "text"}
            <MarkdownRenderer content={block.text} {isStreaming} />
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
        <MarkdownRenderer content={displayContent} {isStreaming} />
      {/if}

      {#if showGenerating}
        <div class="processing-indicator" role="status" aria-live="polite">
          <span class="processing-label">Generating</span>
        </div>
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
    </div>
  </div>

  <div class="bubble-footer" class:actions-visible={editMode}>
    {#if node.query_duration != null}
      <span class="query-time" title="Query time">{formatDuration(node.query_duration)}</span>
    {/if}
    <div class="bubble-actions">
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
  </div>
</div>

<style>
  .assistant-bubble {
    padding: 14px 20px;
    border-radius: 6px;
    user-select: none;
    -webkit-user-select: none;
  }

  .bubble-footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    opacity: 0;
    transition: opacity 120ms ease;
  }

  .assistant-message-wrapper:hover .bubble-footer,
  .bubble-footer.actions-visible {
    opacity: 1;
  }

  .query-time {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.35);
    font-weight: 400;
  }

  .bubble-actions {
    display: flex;
    gap: 4px;
    align-items: center;
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
    background: rgba(255, 255, 255, 0.12);
    color: rgba(255, 255, 255, 0.9);
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
    border-color: rgba(255, 255, 255, 0.25);
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

  .tool-call-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    margin: 2px 0;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 4px;
    color: rgba(255, 255, 255, 0.7);
    font-size: 12px;
    width: fit-content;
    user-select: none;
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
    user-select: text;
    -webkit-user-select: text;
    cursor: text;
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

  .processing-indicator {
    display: flex;
    align-items: center;
    padding: 6px 0;
  }

  .processing-label {
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
    animation: processingShimmer 2s linear infinite;
  }

  @keyframes processingShimmer {
    0% { background-position: -200% center; }
    100% { background-position: 200% center; }
  }

  @media (prefers-reduced-motion: reduce) {
    .processing-label {
      animation: none;
      background: none;
      -webkit-text-fill-color: #96bef0;
    }
  }
</style>
