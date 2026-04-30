<script lang="ts">
  import type { ConversationNode } from "$lib/types/conversation";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import TextChipBar from "$lib/components/ui/TextChipBar.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import SkillEditable from "$lib/components/ui/SkillEditable.svelte";
  import { Trash2, Pencil, Copy, Check } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { handleEditablePaste } from "$lib/utils/paste";
  import { highlightSkills } from "$lib/utils/skillHighlight";
  import { getSkillsStore } from "$lib/stores/skills.svelte";

  let {
    node,
    showDelete = false,
    onContentChange,
    onDelete,
    onRegenerate,
    onRemoveTextAttachment,
    onRemoveImage,
    onAddTextAttachment,
    onAddImage,
  }: {
    node: ConversationNode;
    showDelete: boolean;
    onContentChange: (content: string) => void;
    onDelete: (nodeId: string) => void;
    onRegenerate: () => void;
    onRemoveTextAttachment: (index: number) => void;
    onRemoveImage: (index: number) => void;
    onAddTextAttachment: (text: string) => void;
    onAddImage: (data: string, mediaType: string) => void;
  } = $props();

  const skillsStore = getSkillsStore();

  function classifySkillToken(token: string, _finished: boolean): string | null {
    return skillsStore.nameSet.has(token.slice(1)) ? "skill-badge" : null;
  }

  function formatUserContent(content: string): string {
    return highlightSkills(content, classifySkillToken, "\n");
  }

  let editMode = $state(false);
  let editText = $state("");
  let skillEditable: ReturnType<typeof SkillEditable> | undefined = $state();

  function toggleEditMode() {
    editMode = !editMode;
    if (editMode) {
      editText = node.content;
      requestAnimationFrame(() => {
        if (skillEditable) {
          skillEditable.setTextAndHighlight(editText);
          skillEditable.focus();
          skillEditable.restoreCursor(editText.length);
        }
      });
    }
  }

  function handleEditInput() {
    onContentChange(editText);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      editMode = false;
      onRegenerate();
    }
  }

  async function handlePaste(e: ClipboardEvent) {
    await handleEditablePaste(e, {
      onTextAttachment: onAddTextAttachment,
      onImage: (img) => onAddImage(img.data, img.media_type),
    });
  }

  async function copyContent() {
    await navigator.clipboard.writeText(node.content);
  }
</script>

<div class="user-message-wrapper" class:editing={editMode}>
  <div class="user-bubble" class:editing={editMode}>
    <div class="bubble-body">
      {#if editMode}
        <div class="bubble-edit-field">
          {#if node.text_attachments.length > 0 || node.images.length > 0}
            <div class="attachment-row">
              <TextChipBar textAttachments={node.text_attachments} onremove={onRemoveTextAttachment} />
              <ImageChipBar images={node.images} onremove={onRemoveImage} />
            </div>
          {/if}
          <SkillEditable
            bind:this={skillEditable}
            bind:text={editText}
            editableClass="bubble-editable"
            oninput={handleEditInput}
            onkeydown={handleKeydown}
            onpaste={handlePaste}
          />
        </div>
      {:else}
        {#if node.text_attachments.length > 0 || node.images.length > 0}
          <div class="attachment-row">
            <TextChipBar textAttachments={node.text_attachments} readonly={true} />
            <ImageChipBar images={node.images} readonly={true} />
          </div>
        {/if}
        <div class="bubble-text">{@html formatUserContent(node.content)}</div>
      {/if}
    </div>
  </div>
  <div class="bubble-actions" class:actions-visible={editMode}>
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
  </div>
</div>

<style>
  .user-message-wrapper {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    max-width: 80%;
    margin-left: auto;
  }

  .user-message-wrapper.editing {
    max-width: 100%;
    width: 100%;
    margin-left: 0;
    align-items: stretch;
  }

  .user-bubble {
    padding: 14px 20px;
    background: rgba(74, 158, 187, 0.06);
    border-radius: 12px;
    box-sizing: border-box;
    user-select: none;
    -webkit-user-select: none;
  }

  .attachment-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 2px 0;
  }

  .bubble-text {
    font-size: 14px;
    line-height: 1.5;
    color: #e0e0e0;
    white-space: pre-wrap;
    word-wrap: break-word;
    user-select: text;
    -webkit-user-select: text;
  }

  .bubble-text :global(.skill-badge) {
    display: inline;
    color: rgba(100, 160, 255, 0.9);
  }

  .bubble-actions {
    display: flex;
    justify-content: flex-end;
    gap: 4px;
    padding: 4px 0;
    opacity: 0;
    transition: opacity 120ms ease;
  }

  .user-message-wrapper:hover .bubble-actions,
  .bubble-actions.actions-visible {
    opacity: 1;
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

  .user-bubble.editing :global(.bubble-editable) {
    font-size: 14px;
    line-height: 1.5;
    max-height: 40vh;
    padding: 4px 0 8px;
  }

  .user-bubble.editing :global(.autocomplete-dropdown) {
    bottom: auto;
    top: 100%;
    margin-bottom: 0;
    margin-top: 4px;
  }
</style>
