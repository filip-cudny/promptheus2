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
    background: #3a3a3a;
    border: 1px solid #555;
    border-radius: 12px;
    font-size: 12px;
    color: #f0f0f0;
    white-space: nowrap;
    font-family: inherit;
  }

  .clickable {
    cursor: pointer;
  }

  .clickable:hover {
    background: #454545;
  }

  .chip-disabled {
    background: #2a2a2a;
    border-color: #444;
    color: #666;
    cursor: default;
  }
</style>
