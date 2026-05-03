<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    ref = $bindable<HTMLDivElement | undefined>(),
    children,
    onmousemove,
  }: {
    ref?: HTMLDivElement | undefined;
    children: Snippet;
    onmousemove?: (e: MouseEvent) => void;
  } = $props();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="menu-shell" role="menu" bind:this={ref} {onmousemove}>
  {@render children()}
</div>

<style>
  .menu-shell {
    display: flex;
    flex-direction: column;
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-xl);
    padding: var(--space-2) var(--space-0);
    width: 100%;
    box-sizing: border-box;
    font-family: var(--font-sans);
    font-size: var(--font-size-base);
    color: var(--text-primary);
    overflow-y: hidden;
  }
</style>
