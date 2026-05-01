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
    hoverActions = false,
    actionsVisible = false,
    children,
  }: {
    title: string;
    collapsed: boolean;
    headerClass?: string;
    headerLeft?: Snippet;
    actions?: Snippet;
    hoverActions?: boolean;
    actionsVisible?: boolean;
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
        <span class="collapsible-actions" class:hover-actions={hoverActions} class:actions-visible={actionsVisible} onclick={(e) => e.stopPropagation()}>
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
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .collapsible-header {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    padding: 5px var(--space-6);
    background: var(--surface-overlay-faint);
    border: none;
    color: var(--text-primary);
    font: inherit;
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
    text-align: left;
  }

  .collapsible-header:hover {
    background: var(--surface-overlay);
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
    gap: var(--space-2);
    flex-shrink: 0;
  }

  .collapsible-actions.hover-actions {
    opacity: 0;
    transition: opacity var(--motion-fast) var(--ease-default);
  }

  .collapsible-header:hover .collapsible-actions.hover-actions,
  .collapsible-actions.actions-visible {
    opacity: 1;
  }

  .collapsible-content {
    padding: var(--space-5) var(--space-7);
  }
</style>
