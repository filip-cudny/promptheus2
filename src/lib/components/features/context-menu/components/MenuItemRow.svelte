<script lang="ts">
  import { Mic, Square, X } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  type Props = {
    selected: boolean;
    disabled: boolean;
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
    selected,
    disabled,
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
</div>

<style>
  .menu-item-row {
    display: flex;
    align-items: center;
  }

  .menu-item-row.selected {
    background: var(--surface-overlay);
  }

  .menu-item-row.selected:active {
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
