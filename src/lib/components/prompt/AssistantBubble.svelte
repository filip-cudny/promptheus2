<script lang="ts">
  import type { ConversationNode } from "$lib/types/conversation";
  import CollapsibleSection from "$lib/components/ui/CollapsibleSection.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import MarkdownRenderer from "$lib/components/ui/MarkdownRenderer.svelte";
  import { resizeTextarea } from "$lib/utils/autoResize";
  import { Copy, Check, RefreshCw, Trash2, ChevronLeft, ChevronRight, Pencil } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    node,
    displayContent,
    outputNumber,
    showDelete = false,
    isStreaming = false,
    branchInfo = { current: 1, total: 1 },
    onRegenerate,
    onBranchPrev,
    onBranchNext,
    onContentChange,
    onDelete,
  }: {
    node: ConversationNode;
    displayContent: string;
    outputNumber: number;
    showDelete: boolean;
    isStreaming: boolean;
    branchInfo: { current: number; total: number };
    onRegenerate: (nodeId: string) => void;
    onBranchPrev: (nodeId: string) => void;
    onBranchNext: (nodeId: string) => void;
    onContentChange: (content: string) => void;
    onDelete: (nodeId: string) => void;
  } = $props();

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
    {:else}
      <MarkdownRenderer content={displayContent} {isStreaming} />
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
</style>
