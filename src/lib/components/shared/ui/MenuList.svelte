<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    expand = false,
    role = "listbox",
    children,
  }: {
    expand?: boolean;
    role?: "listbox" | "menu";
    children: Snippet;
  } = $props();
</script>

<div class="menu-list" class:expand role={role === "menu" ? "menu" : "listbox"}>
  {@render children()}
</div>

<style>
  .menu-list {
    display: inline-flex;
    flex-direction: column;
    min-width: 160px;
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--font-size-md);
    overflow: hidden;
  }

  .menu-list.expand {
    display: flex;
    width: 100%;
  }

  .menu-list :global(.menu-list-separator) {
    height: 1px;
    background: var(--surface-overlay);
    margin: var(--space-1) var(--space-4);
  }

  .menu-list :global(.menu-list-info) {
    padding: var(--space-3) var(--space-6);
    color: var(--text-muted);
    font-size: var(--font-size-md);
    line-height: 1.4;
    white-space: normal;
  }

  .menu-list :global(.menu-list-meta-group) {
    padding: var(--space-1) var(--space-0) var(--space-2);
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .menu-list :global(.menu-list-meta) {
    display: flex;
    align-items: baseline;
    gap: var(--space-6);
    padding: var(--space-1) var(--space-6);
    font-size: var(--font-size-sm);
    line-height: var(--line-height-tight);
  }

  .menu-list :global(.menu-list-meta-key) {
    color: var(--text-disabled);
    flex-shrink: 0;
  }

  .menu-list :global(.menu-list-meta-value) {
    color: var(--text-secondary);
    font-family: var(--font-mono);
    margin-left: auto;
    text-align: right;
    word-break: break-all;
  }
</style>
