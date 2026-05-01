<script lang="ts">
  import { ChevronRight } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  type Props = {
    expanded: boolean;
    anchorEl?: HTMLElement;
    onclick: () => void;
    onhover: () => void;
  };

  let {
    expanded,
    anchorEl = $bindable(),
    onclick,
    onhover,
  }: Props = $props();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="menu-item-row" bind:this={anchorEl} onmouseenter={onhover}>
  <button
    class="menu-item settings-toggle"
    role="menuitem"
    tabindex={-1}
    {onclick}
  >
    <span class="settings-chevron" class:expanded>
      <ChevronRight size={ICON_SIZE.sm} />
    </span>
    <span class="item-label">Settings</span>
  </button>
</div>

<style>
  .menu-item-row {
    display: flex;
    align-items: center;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-6);
    border: none;
    background: transparent;
    color: var(--text-primary);
    font: inherit;
    text-align: left;
    cursor: pointer;
    flex: 1;
    min-width: 0;
    box-sizing: border-box;
    border-radius: 0;
    outline: none;
  }

  .settings-toggle {
    gap: var(--space-2);
  }

  .settings-chevron {
    display: flex;
    align-items: center;
    transition: transform var(--motion-default) var(--ease-default);
    color: var(--text-disabled);
  }

  .settings-chevron.expanded {
    transform: rotate(90deg);
  }

  .item-label {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
