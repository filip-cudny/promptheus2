<script lang="ts">
  import type { Snippet } from "svelte";
  import { slide } from "svelte/transition";
  import { ChevronRight, ChevronDown } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    title,
    collapsed = $bindable(false),
    headerClass,
    headerLeft,
    actions,
    children,
  }: {
    title: string;
    collapsed: boolean;
    headerClass?: string;
    headerLeft?: Snippet;
    actions?: Snippet;
    children: Snippet;
  } = $props();
</script>

<div class="collapsible-section">
  <button class="collapsible-header {headerClass ?? ''}" onclick={() => (collapsed = !collapsed)}>
    <span class="collapse-arrow">
      {#if collapsed}
        <ChevronRight size={ICON_SIZE.md} />
      {:else}
        <ChevronDown size={ICON_SIZE.md} />
      {/if}
    </span>
    {#if headerLeft}
      {@render headerLeft()}
    {/if}
    <span class="collapsible-title">{title}</span>
    {#if actions}
      <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
        <span class="collapsible-actions" onclick={(e) => e.stopPropagation()}>
        {@render actions()}
      </span>
    {/if}
  </button>
  {#if !collapsed}
    <div class="collapsible-content" transition:slide={{ duration: 150 }}>
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
    display: flex;
    align-items: center;
    flex-shrink: 0;
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
    padding: 10px 14px;
  }
</style>
