<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    children,
    width = "220px",
  }: {
    children: Snippet;
    width?: string;
  } = $props();
</script>

<aside class="sidebar" style:width>
  {@render children()}
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--surface-sunken);
    border-right: 1px solid var(--border-faint);
    overflow: hidden;
    flex-shrink: 0;
  }
</style>
