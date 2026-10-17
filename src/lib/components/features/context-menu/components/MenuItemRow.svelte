<script lang="ts">
  import type { Snippet } from "svelte";
  import { Mic, Square, X } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  type Props = {
    selected?: boolean;
    disabled?: boolean;
    rowEl?: HTMLElement;
    hoverHighlight?: boolean;
    icon?: Snippet;
    trailing?: Snippet;
    executing?: boolean;
    recording?: boolean;
    iconName?: string | null;
    promptNumber?: number | null;
    label: string;
    onclick: (e: MouseEvent) => void;
    oncontextmenu?: (e: MouseEvent) => void;
    onhover?: () => void;
  };

  let {
    selected = false,
    disabled = false,
    rowEl = $bindable(),
    hoverHighlight = false,
    icon,
    trailing,
    executing = false,
    recording = false,
    iconName = null,
    promptNumber = null,
    label,
    onclick,
    oncontextmenu,
    onhover,
  }: Props = $props();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="menu-item-row"
  class:selected
  class:has-trailing={trailing}
  class:hover-highlight={hoverHighlight}
  bind:this={rowEl}
  onmouseenter={onhover}
  oncontextmenu={oncontextmenu}
>
  <button
    class="menu-item"
    class:disabled
    class:executing
    role="menuitem"
    aria-disabled={disabled}
    tabindex={-1}
    {onclick}
  >
    {#if icon}
      {@render icon()}
    {/if}
    {#if iconName === "square"}
      <span class="item-icon"><Square size={ICON_SIZE.sm} /></span>
    {:else if iconName === "mic"}
      <span class="item-icon"><Mic size={ICON_SIZE.md} /></span>
    {/if}
    {#if promptNumber !== null && promptNumber !== undefined}
      {#if executing}
        <span class="prompt-number executing"><X size={ICON_SIZE.sm} /></span>
      {:else if recording}
        <span class="prompt-number executing"><Square size={ICON_SIZE.sm} /></span>
      {:else}
        <span class="prompt-number">{promptNumber}.</span>
      {/if}
    {/if}
    <span class="item-label">{label}</span>
  </button>
  {#if trailing}
    {@render trailing()}
  {/if}
</div>

<style>
  .menu-item-row {
    display: flex;
    align-items: center;
  }

  .menu-item-row.has-trailing {
    padding-right: var(--space-4);
  }

  .menu-item-row.has-trailing .menu-item {
    padding-right: var(--space-2);
  }

  .menu-item-row.selected,
  .menu-item-row.hover-highlight:hover {
    background: var(--surface-overlay);
  }

  .menu-item-row.selected:active,
  .menu-item-row.hover-highlight:active {
    background: rgba(255, 255, 255, 0.15);
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

  .menu-item.disabled {
    color: var(--text-disabled);
    cursor: default;
  }

  .item-icon {
    flex-shrink: 0;
    width: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .prompt-number {
    flex-shrink: 0;
    min-width: 20px;
    text-align: right;
    color: var(--text-faint);
    font-size: var(--font-size-md);
    margin-left: -4px;
  }

  .prompt-number.executing {
    color: var(--text-primary);
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .item-label {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
