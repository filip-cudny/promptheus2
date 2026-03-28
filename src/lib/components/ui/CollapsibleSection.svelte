<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    collapsed = $bindable(false),
    actions,
    children,
  }: {
    title: string;
    collapsed: boolean;
    actions?: Snippet;
    children: Snippet;
  } = $props();
</script>

<div class="collapsible-section">
  <button class="collapsible-header" onclick={() => (collapsed = !collapsed)}>
    <span class="collapse-arrow" class:rotated={!collapsed}>▶</span>
    <span class="collapsible-title">{title}</span>
    {#if actions}
      <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
        <span class="collapsible-actions" onclick={(e) => e.stopPropagation()}>
        {@render actions()}
      </span>
    {/if}
  </button>
  {#if !collapsed}
    <div class="collapsible-content">
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .collapsible-section {
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    overflow: hidden;
  }

  .collapsible-header {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    background: rgba(255, 255, 255, 0.04);
    border: none;
    color: #e0e0e0;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    text-align: left;
  }

  .collapsible-header:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .collapse-arrow {
    font-size: 10px;
    transition: transform 150ms ease;
    flex-shrink: 0;
  }

  .collapse-arrow.rotated {
    transform: rotate(90deg);
  }

  .collapsible-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .collapsible-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .collapsible-content {
    padding: 8px 12px;
  }
</style>
