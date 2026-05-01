<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { save } from "@tauri-apps/plugin-dialog";
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

  async function handleSaveSvg(svg: string) {
    const path = await save({
      defaultPath: "mermaid-diagram.svg",
      filters: [{ name: "SVG", extensions: ["svg"] }],
    });
    if (path) await invoke("write_text_file", { path, content: svg });
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
            <MarkdownRenderer content={block.text} {isStreaming} onopen={openUrl} onsavesvg={handleSaveSvg} />
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
        <MarkdownRenderer content={displayContent} {isStreaming} onopen={openUrl} onsavesvg={handleSaveSvg} />
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
    padding: var(--space-7) var(--space-10);
    border-radius: var(--radius-lg);
    user-select: none;
    -webkit-user-select: none;
  }

  .bubble-footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-0);
    opacity: 0;
    transition: opacity var(--motion-fast) var(--ease-default);
  }

  .assistant-message-wrapper:hover .bubble-footer,
  .bubble-footer.actions-visible {
    opacity: 1;
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

  .branch-nav {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .branch-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--text-muted);
    border-radius: var(--radius-md);
    padding: var(--space-1);
    cursor: pointer;
  }

  .branch-btn:hover:not(:disabled) {
    background: var(--surface-overlay);
    color: var(--text-secondary);
  }

  .branch-btn:disabled {
    opacity: var(--opacity-disabled);
    cursor: default;
  }

  .branch-counter {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    min-width: 24px;
    text-align: center;
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

  .bubble-edit-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    padding: var(--space-4) var(--space-4) var(--space-0);
  }

  .bubble-edit-field:focus-within {
    border-color: var(--border-strong);
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

  .tool-call-chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    margin: var(--space-1) var(--space-0);
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: var(--font-size-md);
    width: fit-content;
    user-select: none;
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-top: var(--space-3);
    padding: var(--space-4) var(--space-5);
    background: var(--danger-bg-soft);
    border-radius: var(--radius-md);
    color: var(--danger);
    font-size: var(--font-size-md);
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
    gap: var(--space-2);
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

  .cancelled-hint {
    display: block;
    margin-top: var(--space-3);
    font-size: var(--font-size-sm);
    font-style: italic;
    color: var(--text-disabled);
  }

  .processing-indicator {
    display: flex;
    align-items: center;
    padding: var(--space-3) var(--space-0);
  }

  .processing-label {
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-semibold);
    background: linear-gradient(
      90deg,
      var(--accent-ring) 0%,
      var(--accent) 50%,
      var(--accent-ring) 100%
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
      -webkit-text-fill-color: var(--accent);
    }
  }
</style>
