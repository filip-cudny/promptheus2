<script lang="ts">
  import type { ContextItem } from "$lib/types/context";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";
  import Chip from "$lib/components/shared/ui/Chip.svelte";
  import ImageChipBar from "$lib/components/shared/ui/ImageChipBar.svelte";
  import {
    FileSymlink,
    FilePlus,
    Pencil,
    Copy,
    Trash2,
    Check,
    X,
  } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    items,
    onReplaceFromClipboard,
    onAppendFromClipboard,
    onOpenEditor,
    onCopyAll,
    onClear,
    onRemoveItem,
    onOpenImagePreview,
  }: {
    items: ContextItem[];
    onReplaceFromClipboard: () => Promise<void>;
    onAppendFromClipboard: () => Promise<void>;
    onOpenEditor: () => Promise<void>;
    onCopyAll: () => Promise<void>;
    onClear: () => Promise<void>;
    onRemoveItem: (index: number) => Promise<boolean | void>;
    onOpenImagePreview: (data: string, mediaType: string) => void;
  } = $props();

  let hasTextItems = $derived(items.some((i) => i.item_type === "text"));
  let isEmpty = $derived(items.length === 0);
  let copyConfirm = $state<number | null>(null);

  function truncateText(text: string, maxLength = 50): string {
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + "\u2026";
  }

  async function handleChipCopy(idx: number, content: string) {
    await navigator.clipboard.writeText(content);
    copyConfirm = idx;
    setTimeout(() => (copyConfirm = null), 1200);
  }
</script>

<div class="context-section">
  <div class="section-header">
    <span class="header-label">
      Context
      {#if !isEmpty}
        <span class="badge">{items.length}</span>
      {/if}
    </span>
    <span class="header-actions">
      <ActionIconButton
        icon={FileSymlink}
        confirmIcon={Check}
        onclick={onReplaceFromClipboard}
        title="Replace context with clipboard"
      />
      <ActionIconButton
        icon={FilePlus}
        confirmIcon={Check}
        onclick={onAppendFromClipboard}
        title="Append clipboard to context"
      />
      <ActionIconButton
        icon={Pencil}
        onclick={onOpenEditor}
        title="Edit context"
      />
      <ActionIconButton
        icon={Copy}
        confirmIcon={Check}
        onclick={onCopyAll}
        title="Copy context text"
        disabled={!hasTextItems}
      />
      <ActionIconButton
        icon={Trash2}
        confirmIcon={Check}
        onclick={onClear}
        title="Clear all context"
        disabled={isEmpty}
      />
    </span>
  </div>

  {#if !isEmpty}
    <div class="chips">
      {#each items as item, idx}
        {#if item.item_type === "text"}
          <Chip title={item.content} onclick={() => handleChipCopy(idx, item.content)}>
            <span class="chip-copy">
              {#if copyConfirm === idx}
                <Check size={ICON_SIZE.md} />
              {:else}
                <Copy size={ICON_SIZE.md} />
              {/if}
            </span>
            <span class="chip-text">{truncateText(item.content)}</span>
            <button class="chip-remove" onclick={(e) => { e.stopPropagation(); onRemoveItem(idx); }}>
              <X size={11} strokeWidth={2.5} />
            </button>
          </Chip>
        {:else if item.item_type === "image"}
          <ImageChipBar
            images={[{ data: item.data, media_type: item.media_type }]}
            variant="small"
            onremove={() => onRemoveItem(idx)}
            onopen={(image) => onOpenImagePreview(image.data, image.media_type)}
          />
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .context-section {
    padding: var(--space-1) var(--space-0);
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--space-2) var(--space-6);
    border: none;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--font-size-sm);
    text-transform: capitalize;
    letter-spacing: var(--tracking-label);
    box-sizing: border-box;
  }

  .header-label {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .badge {
    background: rgba(255, 255, 255, 0.15);
    border-radius: var(--radius-xl);
    padding: var(--space-0) var(--space-3);
    font-size: var(--font-size-xs);
    line-height: 16px;
  }

  .header-actions {
    display: flex;
    gap: var(--space-1);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-6) var(--space-1);
    overflow: hidden;
  }

  .chips :global(.chip) {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chip-copy {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .chip-text {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chip-remove {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    padding: var(--space-1);
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--text-disabled);
    cursor: pointer;
  }

  .chip-remove:hover {
    background: rgba(255, 255, 255, 0.15);
    color: var(--text-secondary);
  }

</style>
