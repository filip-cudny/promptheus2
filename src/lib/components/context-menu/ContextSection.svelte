<script lang="ts">
  import type { ContextItem } from "$lib/types/context";
  import {
    clearContext,
    getContextText,
    setContextFromClipboard,
    appendContextFromClipboard,
  } from "$lib/services/context";
  import { openContextEditor } from "$lib/services/contextEditor";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import {
    FileSymlink,
    FilePlus,
    Pencil,
    Copy,
    Trash2,
    Check,
    FileText,
  } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let { items }: { items: ContextItem[] } = $props();

  let hasTextItems = $derived(items.some((i) => i.item_type === "text"));
  let imageItems = $derived(
    items
      .filter((i): i is ContextItem & { item_type: "image" } => i.item_type === "image")
      .map((i) => ({ data: i.data, media_type: i.media_type })),
  );
  let isEmpty = $derived(items.length === 0);

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
      {#each items as item}
        {#if item.item_type === "text"}
          <span class="chip chip-text" title={item.content}>
            <FileText size={ICON_SIZE.md} />
            {truncateText(item.content)}
          </span>
        {/if}
      {/each}
      {#if imageItems.length > 0}
        <ImageChipBar images={imageItems} readonly={true} />
      {/if}
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
    text-transform: uppercase;
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
    gap: 4px;
    padding: 4px 12px 6px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    background: #3a3a3a;
    border: 1px solid #555;
    border-radius: 12px;
    font-size: 12px;
    color: #f0f0f0;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

</style>
