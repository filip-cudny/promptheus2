<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title = "",
    disabled = false,
    onclick,
    children,
  }: {
    title?: string;
    disabled?: boolean;
    onclick?: (e: MouseEvent) => void;
    children: Snippet;
  } = $props();
</script>

{#if onclick}
  <button class="chip" class:clickable={!disabled} class:chip-disabled={disabled} {title} {disabled} {onclick}>
    {@render children()}
  </button>
{:else}
  <span class="chip" class:chip-disabled={disabled} {title}>
    {@render children()}
  </span>
{/if}

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    background: #363636;
    border: 1px solid #4e4e4e;
    border-radius: 12px;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.78);
    white-space: nowrap;
    font-family: inherit;
  }

  .clickable {
    cursor: pointer;
  }

  .clickable:hover {
    background: #383838;
  }

  .chip-disabled {
    background: #2a2a2a;
    border-color: #444;
    color: #666;
    cursor: default;
  }
</style>
