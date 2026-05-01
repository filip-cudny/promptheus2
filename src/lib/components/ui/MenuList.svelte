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

  .menu-list :global(.menu-list-item) {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--text-primary);
    padding: 7px 12px;
    text-align: left;
    cursor: pointer;
    font: inherit;
    line-height: 1.35;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    box-sizing: border-box;
  }

  .menu-list :global(.menu-list-item:hover:not(:disabled):not(.is-disabled)) {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }

  .menu-list :global(.menu-list-item.is-active) {
    color: var(--text-primary);
    background: var(--surface-overlay-faint);
  }

  .menu-list :global(.menu-list-item:disabled),
  .menu-list :global(.menu-list-item.is-disabled) {
    color: var(--text-disabled);
    cursor: default;
  }

  .menu-list :global(.menu-list-label) {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .menu-list :global(.menu-list-shortcut) {
    color: var(--text-muted);
    padding-left: 8px;
    display: inline-flex;
    align-items: center;
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

  .menu-list :global(.menu-list-icon) {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-primary);
  }

  .menu-list :global(.menu-list-icon svg) {
    width: 100%;
    height: 100%;
    display: block;
  }

  .menu-list :global(.menu-list-icon img) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: contain;
  }
</style>
