<script lang="ts">
  import type { ContextItem } from "$lib/types/context";
  import {
    clearContext,
    getContextText,
    setContextFromClipboard,
    appendContextFromClipboard,
    removeContextItem,
  } from "$lib/services/context";
  import { openContextEditor } from "$lib/services/contextEditor";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import Chip from "$lib/components/ui/Chip.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import {
    FileSymlink,
    FilePlus,
    Pencil,
    Copy,
    Trash2,
    Check,
    FileText,
    X,
  } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let { items }: { items: ContextItem[] } = $props();

  let hasTextItems = $derived(items.some((i) => i.item_type === "text"));
  let isEmpty = $derived(items.length === 0);
  let copyConfirm = $state<number | null>(null);

  function truncateText(text: string, maxLength = 50): string {
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + "\u2026";
  }

  async function handleCopy() {
    const text = await getContextText();
    if (text) {
      await navigator.clipboard.writeText(text);
    }
  }

  async function handleChipCopy(idx: number, content: string) {
    await navigator.clipboard.writeText(content);
    copyConfirm = idx;
    setTimeout(() => (copyConfirm = null), 1200);
  }

  async function handleClear() {
    await clearContext();
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
        onclick={setContextFromClipboard}
        title="Replace context with clipboard"
      />
      <ActionIconButton
        icon={FilePlus}
        confirmIcon={Check}
        onclick={appendContextFromClipboard}
        title="Append clipboard to context"
      />
      <ActionIconButton
        icon={Pencil}
        onclick={openContextEditor}
        title="Edit context"
      />
      <ActionIconButton
        icon={Copy}
        confirmIcon={Check}
        onclick={handleCopy}
        title="Copy context text"
        disabled={!hasTextItems}
      />
      <ActionIconButton
        icon={Trash2}
        confirmIcon={Check}
        onclick={handleClear}
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
            <button class="chip-remove" onclick={(e) => { e.stopPropagation(); removeContextItem(idx); }}>
              <X size={11} strokeWidth={2.5} />
            </button>
          </Chip>
        {:else if item.item_type === "image"}
          <ImageChipBar
            images={[{ data: item.data, media_type: item.media_type }]}
            variant="small"
            onremove={() => removeContextItem(idx)}
          />
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .context-section {
    padding: 2px 0;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 4px 12px;
    border: none;
    background: transparent;
    color: rgba(255, 255, 255, 0.6);
    font: inherit;
    font-size: 11px;
    text-transform: capitalize;
    letter-spacing: 0.5px;
    box-sizing: border-box;
  }

  .header-label {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .badge {
    background: rgba(255, 255, 255, 0.15);
    border-radius: 8px;
    padding: 0 6px;
    font-size: 10px;
    line-height: 16px;
  }

  .header-actions {
    display: flex;
    gap: 2px;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    padding: 4px 12px 2px;
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
    padding: 2px;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: rgba(255, 255, 255, 0.4);
    cursor: pointer;
  }

  .chip-remove:hover {
    background: rgba(255, 255, 255, 0.15);
    color: rgba(255, 255, 255, 0.8);
  }

</style>
