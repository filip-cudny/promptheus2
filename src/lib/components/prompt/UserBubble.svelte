<script lang="ts">
  import type { ConversationNode } from "$lib/types/conversation";
  import CollapsibleSection from "$lib/components/ui/CollapsibleSection.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import { resizeTextarea } from "$lib/utils/autoResize";
  import { Trash2, Pencil, Copy, Check } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    node,
    messageNumber,
    showDelete = false,
    onContentChange,
    onDelete,
    onRegenerate,
  }: {
    node: ConversationNode;
    messageNumber: number;
    showDelete: boolean;
    onContentChange: (content: string) => void;
    onDelete: (nodeId: string) => void;
    onRegenerate: () => void;
  } = $props();

  let collapsed = $state(false);
  let editMode = $state(false);
  let textarea: HTMLTextAreaElement | undefined = $state();

  $effect(() => {
    node.content;
    if (editMode && textarea) {
      requestAnimationFrame(() => resizeTextarea(textarea!));
    }
  });

  function toggleEditMode() {
    editMode = !editMode;
    if (editMode) {
      requestAnimationFrame(() => {
        if (textarea) resizeTextarea(textarea!);
        textarea?.focus();
      });
    }
  }

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    onContentChange(target.value);
    resizeTextarea(target);
  }

  async function copyContent() {
    await navigator.clipboard.writeText(node.content);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      editMode = false;
      onRegenerate();
    }
  }
</script>

<div class="user-bubble">
  <CollapsibleSection title="" bind:collapsed hoverActions actionsVisible={editMode}>
    {#snippet headerLeft()}
      <span class="role-badge user-badge">Me</span>
      <span class="turn-number"># {messageNumber}</span>
    {/snippet}
    {#snippet actions()}
      <ActionIconButton
        icon={Copy}
        confirmIcon={Check}
        onclick={copyContent}
        title="Copy text"
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
        {#if node.images.length > 0}
          <ImageChipBar images={node.images} readonly={true} />
        {/if}
        <textarea
          bind:this={textarea}
          value={node.content}
          oninput={handleInput}
          onkeydown={handleKeydown}
          class="bubble-textarea"
          rows="1"
        ></textarea>
      </div>
    {:else}
      {#if node.images.length > 0}
        <ImageChipBar images={node.images} readonly={true} />
      {/if}
      <div class="bubble-text">{node.content}</div>
    {/if}
  </CollapsibleSection>
</div>

<style>
  .user-bubble {
    border-left: 3.5px solid #4a9ebb;
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

  .user-badge {
    background: rgba(74, 158, 187, 0.25);
    color: #7dd3f0;
  }

  .turn-number {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.4);
    font-weight: 500;
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
    background: rgba(74, 158, 187, 0.2);
    color: #7dd3f0;
  }

  .delete-btn:hover {
    background: rgba(200, 60, 60, 0.3);
    border-color: rgba(200, 60, 60, 0.5);
    color: #ff8a8a;
  }

  .bubble-text {
    font-size: 14px;
    line-height: 1.5;
    color: #e0e0e0;
    white-space: pre-wrap;
    word-wrap: break-word;
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
    border-color: rgba(74, 158, 187, 0.4);
  }

  .bubble-textarea {
    width: 100%;
    background: transparent;
    border: none;
    color: #e0e0e0;
    font-family: inherit;
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
