<script lang="ts">
  import type { ConversationNode } from "$lib/types/conversation";
  import CollapsibleSection from "$lib/components/ui/CollapsibleSection.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import TextChipBar from "$lib/components/ui/TextChipBar.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import SkillEditable from "$lib/components/ui/SkillEditable.svelte";
  import { Trash2, Pencil, Copy, Check } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { highlightSkills } from "$lib/utils/skillHighlight";
  import { getSkillsStore } from "$lib/stores/skills.svelte";

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

  const skillsStore = getSkillsStore();

  function isKnownSkill(name: string): boolean {
    return skillsStore.nameSet.has(name.slice(1));
  }

  function formatUserContent(content: string): string {
    return highlightSkills(content, isKnownSkill, "skill-badge", "\n");
  }

  let collapsed = $state(false);
  let editMode = $state(false);
  let editText = $state("");
  let skillEditable: ReturnType<typeof SkillEditable> | undefined = $state();

  function toggleEditMode() {
    editMode = !editMode;
    if (editMode) {
      editText = node.content;
      requestAnimationFrame(() => {
        if (skillEditable) {
          skillEditable.setTextAndHighlight(node.content);
          skillEditable.focus();
          skillEditable.restoreCursor(node.content.length);
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

  async function copyContent() {
    await navigator.clipboard.writeText(node.content);
  }
</script>

<div class="user-bubble" class:editing={editMode}>
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
        {#if node.text_attachments.length > 0 || node.images.length > 0}
          <div class="attachment-row">
            <TextChipBar textAttachments={node.text_attachments} readonly={true} />
            <ImageChipBar images={node.images} readonly={true} />
          </div>
        {/if}
        <SkillEditable
          bind:this={skillEditable}
          bind:text={editText}
          editableClass="bubble-editable"
          oninput={handleEditInput}
          onkeydown={handleKeydown}
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
  </CollapsibleSection>
</div>

<style>
  .attachment-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 2px 0;
  }

  .user-bubble {
    border-left: 3.5px solid #4a9ebb;
    border-radius: 6px;
  }

  .user-bubble.editing :global(.collapsible-section) {
    overflow: visible;
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

  .bubble-text :global(.skill-badge) {
    display: inline;
    font-weight: 600;
    color: rgba(100, 160, 255, 0.9);
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

  .bubble-edit-field :global(.bubble-editable) {
    font-size: 14px;
    line-height: 1.5;
    max-height: 40vh;
    padding: 4px 0 8px;
  }

  .bubble-edit-field :global(.autocomplete-dropdown) {
    bottom: auto;
    top: 100%;
    margin-bottom: 0;
    margin-top: 4px;
  }
</style>
