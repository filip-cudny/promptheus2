<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import type { ConversationNode, ConversationImage } from "$lib/types/conversation";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import SkillEditable from "$lib/components/ui/SkillEditable.svelte";
  import AttachmentRow from "./components/AttachmentRow.svelte";
  import BubbleEditField from "./components/BubbleEditField.svelte";
  import BubbleActionsFooter from "./components/BubbleActionsFooter.svelte";
  import { Trash2, Pencil, Copy, Check, SendHorizonal } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { handleEditablePaste } from "$lib/utils/paste";
  import { highlightSkills } from "$lib/utils/skillHighlight";

  let {
    node,
    showDelete = false,
    classifyToken,
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
    classifyToken: (token: string, finished: boolean) => string | null;
    onContentChange: (content: string) => void;
    onDelete: (nodeId: string) => void;
    onRegenerate: () => void;
    onRemoveTextAttachment: (index: number) => void;
    onRemoveImage: (index: number) => void;
    onAddTextAttachment: (text: string) => void;
    onAddImage: (data: string, mediaType: string) => void;
  } = $props();

  function formatUserContent(content: string): string {
    return highlightSkills(content, classifyToken, "\n");
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

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submitEdit();
    }
  }

  function submitEdit() {
    onContentChange(editText);
    editMode = false;
    onRegenerate();
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

  function openTextPreview(text: string, index: number) {
    const sourceWindow = getCurrentWebviewWindow().label;
    invoke("open_text_preview", { text, index, sourceWindow }).catch((e) =>
      console.error("open_text_preview failed:", e),
    );
  }

  function openImagePreview(image: ConversationImage) {
    invoke("open_image_preview", { data: image.data, mediaType: image.media_type });
  }
</script>

<div class="user-message-wrapper" class:editing={editMode}>
  <div class="user-bubble" class:editing={editMode}>
    <div class="bubble-body">
      {#if editMode}
        <BubbleEditField variant="user">
          <AttachmentRow
            textAttachments={node.text_attachments}
            images={node.images}
            onRemoveText={onRemoveTextAttachment}
            onRemoveImage={onRemoveImage}
            onOpenText={openTextPreview}
            onOpenImage={openImagePreview}
          />
          <SkillEditable
            bind:this={skillEditable}
            bind:text={editText}
            editableClass="bubble-editable"
            onkeydown={handleKeydown}
            onpaste={handlePaste}
          />
        </BubbleEditField>
      {:else}
        <AttachmentRow
          textAttachments={node.text_attachments}
          images={node.images}
          readonly={true}
          onOpenText={openTextPreview}
          onOpenImage={openImagePreview}
        />
        <div class="bubble-text">{@html formatUserContent(node.content)}</div>
      {/if}
    </div>
  </div>
  <BubbleActionsFooter visible={editMode}>
    <ActionIconButton
      icon={Copy}
      confirmIcon={Check}
      onclick={copyContent}
      title="Copy text"
    />
    <button class="icon-btn" class:active={editMode} onclick={toggleEditMode} title={editMode ? "View" : "Edit"}>
      <Pencil size={ICON_SIZE.md} />
    </button>
    {#if editMode}
      <ActionIconButton
        icon={SendHorizonal}
        onclick={submitEdit}
        title="Send (Enter)"
      />
    {/if}
    {#if showDelete}
      <button class="icon-btn delete-btn" onclick={() => onDelete(node.node_id)} title="Delete">
        <Trash2 size={ICON_SIZE.md} />
      </button>
    {/if}
  </BubbleActionsFooter>
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
    margin-left: var(--space-0);
    align-items: stretch;
  }

  .user-bubble {
    padding: var(--space-7) var(--space-10);
    background: rgba(74, 158, 187, 0.06);
    border-radius: var(--radius-2xl);
    box-sizing: border-box;
    user-select: none;
    -webkit-user-select: none;
  }

  .bubble-text {
    font-size: var(--font-size-lg);
    line-height: var(--line-height-normal);
    color: var(--text-primary);
    white-space: pre-wrap;
    word-wrap: break-word;
    user-select: text;
    -webkit-user-select: text;
  }

  .bubble-text :global(.skill-badge) {
    display: inline;
    color: var(--accent);
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
    background: rgba(74, 158, 187, 0.2);
    color: var(--info);
  }

  .delete-btn:hover {
    background: var(--danger-border);
    border-color: var(--danger-border);
    color: var(--danger);
  }

  .user-bubble.editing :global(.bubble-editable) {
    font-size: var(--font-size-lg);
    line-height: var(--line-height-normal);
    max-height: 40vh;
    padding: var(--space-2) var(--space-0) var(--space-4);
  }

  .user-bubble.editing :global(.autocomplete-dropdown) {
    bottom: auto;
    top: 100%;
    margin-bottom: var(--space-0);
    margin-top: var(--space-2);
  }
</style>
