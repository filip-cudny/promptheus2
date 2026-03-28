<script lang="ts">
  import type { ContextItem } from "$lib/types/context";
  import { clearContext, getContextText } from "$lib/services/context";

  let { items }: { items: ContextItem[] } = $props();

  let expanded = $state(true);

  function truncateText(text: string, maxLength = 50): string {
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + "…";
  }

  function formatMediaType(mediaType: string): string {
    const parts = mediaType.split("/");
    return (parts[1] ?? parts[0]).toUpperCase();
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

  let hasTextItems = $derived(items.some((i) => i.item_type === "text"));
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
      <span class="toggle">{expanded ? "▾" : "▸"}</span>
      Context
      <span class="badge">{items.length}</span>
    </span>
    <span class="header-actions">
      {#if hasTextItems}
        <button
          class="action-btn"
          onclick={(e) => { e.stopPropagation(); handleCopy(); }}
          title="Copy context text"
        >
          Copy
        </button>
      {/if}
      <button
        class="action-btn action-btn-clear"
        onclick={(e) => { e.stopPropagation(); handleClear(); }}
        title="Clear all context"
      >
        Clear
      </button>
    </span>
  </div>

  {#if expanded}
    <div class="chips">
      {#each items as item}
        {#if item.item_type === "text"}
          <span class="chip chip-text" title={item.content}>
            {truncateText(item.content)}
          </span>
        {:else if item.item_type === "image"}
          <span class="chip chip-image">
            🖼 {formatMediaType(item.media_type)}
          </span>
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
    font-size: 10px;
    width: 10px;
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
    padding: 1px 6px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    font-size: 10px;
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
