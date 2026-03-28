<script lang="ts">
  import type { ContextItem } from "$lib/types/context";
  import { clearContext, getContextText } from "$lib/services/context";
  import { Copy, Trash2, Check, ChevronDown, ChevronRight, FileText, Image } from "lucide-svelte";

  let { items }: { items: ContextItem[] } = $props();

  let expanded = $state(true);
  let copyConfirm = $state(false);
  let clearConfirm = $state(false);

  function truncateText(text: string, maxLength = 50): string {
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + "\u2026";
  }

  function formatMediaType(mediaType: string): string {
    const parts = mediaType.split("/");
    return (parts[1] ?? parts[0]).toUpperCase();
  }

  function showConfirm(setter: (v: boolean) => void) {
    setter(true);
    setTimeout(() => setter(false), 1200);
  }

  async function handleCopy() {
    const text = await getContextText();
    if (text) {
      await navigator.clipboard.writeText(text);
      showConfirm((v) => (copyConfirm = v));
    }
  }

  async function handleClear() {
    await clearContext();
    showConfirm((v) => (clearConfirm = v));
  }

  let hasTextItems = $derived(items.some((i) => i.item_type === "text"));
  let isEmpty = $derived(items.length === 0);
</script>

<div class="context-section">
  <div
    class="section-header"
    role="button"
    tabindex="-1"
    onclick={() => (expanded = !expanded)}
    onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") expanded = !expanded; }}
  >
    <span class="header-label">
      <span class="toggle">
        {#if expanded}
          <ChevronDown size={12} />
        {:else}
          <ChevronRight size={12} />
        {/if}
      </span>
      Context
      {#if !isEmpty}
        <span class="badge">{items.length}</span>
      {/if}
    </span>
    {#if !isEmpty}
      <span class="header-actions">
        {#if hasTextItems}
          <button
            class="action-btn"
            onclick={(e) => { e.stopPropagation(); handleCopy(); }}
            title="Copy context text"
          >
            {#if copyConfirm}
              <Check size={12} />
            {:else}
              <Copy size={12} />
            {/if}
          </button>
        {/if}
        <button
          class="action-btn action-btn-clear"
          onclick={(e) => { e.stopPropagation(); handleClear(); }}
          title="Clear all context"
        >
          {#if clearConfirm}
            <Check size={12} />
          {:else}
            <Trash2 size={12} />
          {/if}
        </button>
      </span>
    {/if}
  </div>

  {#if expanded}
    {#if isEmpty}
      <div class="empty-hint">No context set</div>
    {:else}
      <div class="chips">
        {#each items as item}
          {#if item.item_type === "text"}
            <span class="chip chip-text" title={item.content}>
              <FileText size={12} />
              {truncateText(item.content)}
            </span>
          {:else if item.item_type === "image"}
            <span class="chip chip-image">
              <Image size={12} />
              {formatMediaType(item.media_type)}
            </span>
          {/if}
        {/each}
      </div>
    {/if}
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
    cursor: pointer;
    box-sizing: border-box;
  }

  .section-header:hover {
    color: rgba(255, 255, 255, 0.8);
  }

  .header-label {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .toggle {
    display: flex;
    align-items: center;
    width: 12px;
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
    gap: 4px;
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 3px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
  }

  .action-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.8);
  }

  .action-btn-clear:hover {
    border-color: rgba(255, 100, 100, 0.4);
    color: rgba(255, 100, 100, 0.8);
  }

  .empty-hint {
    padding: 4px 12px 6px 30px;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.25);
    font-style: italic;
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

  .chip-image {
    color: rgba(255, 255, 255, 0.7);
  }
</style>
