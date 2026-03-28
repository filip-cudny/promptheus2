<script lang="ts">
  import type { ConversationNode } from "$lib/types/conversation";
  import CollapsibleSection from "$lib/components/ui/CollapsibleSection.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import { Trash2 } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    node,
    messageNumber,
    showDelete = false,
    onContentChange,
    onDelete,
  }: {
    node: ConversationNode;
    messageNumber: number;
    showDelete: boolean;
    onContentChange: (content: string) => void;
    onDelete: (nodeId: string) => void;
  } = $props();

  let collapsed = $state(false);
  let textarea: HTMLTextAreaElement | undefined = $state();

  function autoResize() {
    if (!textarea) return;
    textarea.style.height = "auto";
    textarea.style.height = textarea.scrollHeight + "px";
  }

  $effect(() => {
    node.content;
    if (textarea) {
      requestAnimationFrame(autoResize);
    }
  });

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    onContentChange(target.value);
    autoResize();
  }
</script>

<div class="user-bubble">
  <CollapsibleSection title="" bind:collapsed>
    {#snippet headerLeft()}
      <span class="role-badge user-badge">Me</span>
      <span class="turn-number"># {messageNumber}</span>
    {/snippet}
    {#snippet actions()}
      {#if showDelete}
        <button class="icon-btn delete-btn" onclick={() => onDelete(node.node_id)} title="Delete">
          <Trash2 size={ICON_SIZE.md} />
        </button>
      {/if}
    {/snippet}

    {#if node.images.length > 0}
      <ImageChipBar images={node.images} readonly={true} />
    {/if}
    <textarea
      bind:this={textarea}
      value={node.content}
      oninput={handleInput}
      class="bubble-textarea"
      rows="1"
    ></textarea>
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

  .delete-btn:hover {
    background: rgba(200, 60, 60, 0.3);
    border-color: rgba(200, 60, 60, 0.5);
    color: #ff8a8a;
  }

  .bubble-textarea {
    width: 100%;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 4px;
    color: #e0e0e0;
    font-family: inherit;
    font-size: 14px;
    line-height: 1.5;
    padding: 8px;
    resize: none;
    overflow: hidden;
    box-sizing: border-box;
  }

  .bubble-textarea:focus {
    outline: none;
    border-color: rgba(74, 158, 187, 0.4);
  }
</style>
