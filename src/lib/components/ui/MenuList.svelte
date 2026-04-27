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
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 12px;
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
    color: rgba(255, 255, 255, 0.85);
    padding: 6px 12px;
    text-align: left;
    cursor: pointer;
    font: inherit;
    line-height: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    box-sizing: border-box;
  }

  .menu-list :global(.menu-list-item:hover:not(:disabled):not(.is-disabled)) {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }

  .menu-list :global(.menu-list-item.is-active) {
    color: #fff;
    background: rgba(255, 255, 255, 0.06);
  }

  .menu-list :global(.menu-list-item:disabled),
  .menu-list :global(.menu-list-item.is-disabled) {
    color: rgba(255, 255, 255, 0.3);
    cursor: default;
  }

  .menu-list :global(.menu-list-label) {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .menu-list :global(.menu-list-shortcut) {
    color: rgba(255, 255, 255, 0.35);
    font-size: 11px;
    padding-left: 8px;
  }

  .menu-list :global(.menu-list-separator) {
    height: 1px;
    background: rgba(255, 255, 255, 0.1);
    margin: 2px 8px;
  }

  .menu-list :global(.menu-list-info) {
    padding: 6px 12px;
    color: rgba(255, 255, 255, 0.6);
    font-size: 12px;
    line-height: 1.4;
    white-space: normal;
  }

  .menu-list :global(.menu-list-icon) {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: #fff;
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
