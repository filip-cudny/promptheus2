<script lang="ts">
  import type { Component, SvelteComponent } from "svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  type IconComponent =
    | Component<{ size?: number | string }>
    | (new (...args: any[]) => SvelteComponent<{ size?: number | string }>);

  let {
    icon,
    confirmIcon,
    onclick,
    title = "",
    disabled = false,
    size = ICON_SIZE.md,
  }: {
    icon: IconComponent;
    confirmIcon?: IconComponent;
    onclick: (e: MouseEvent) => void;
    title?: string;
    disabled?: boolean;
    size?: number;
  } = $props();

  let confirmed = $state(false);

  function handleClick(e: MouseEvent) {
    e.stopPropagation();
    onclick(e);
    if (confirmIcon) {
      confirmed = true;
      setTimeout(() => (confirmed = false), 1200);
    }
  }
</script>

<button class="action-icon-btn" {title} {disabled} onclick={handleClick}>
  {#if confirmed && confirmIcon}
    <svelte:component this={confirmIcon} {size} />
  {:else}
    <svelte:component this={icon} {size} />
  {/if}
</button>

<style>
  .action-icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
  }

  .action-icon-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.8);
  }

  .action-icon-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
