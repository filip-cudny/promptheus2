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
    padding: 7px 12px;
    text-align: left;
    cursor: pointer;
    font: inherit;
    line-height: 1.35;
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
    color: rgba(255, 255, 255, 0.5);
    padding-left: 8px;
    display: inline-flex;
    align-items: center;
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

  .menu-list :global(.menu-list-meta-group) {
    padding: 2px 0 4px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .menu-list :global(.menu-list-meta) {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding: 2px 12px;
    font-size: 11px;
    line-height: 1.3;
  }

  .menu-list :global(.menu-list-meta-key) {
    color: rgba(255, 255, 255, 0.35);
    flex-shrink: 0;
  }

  .menu-list :global(.menu-list-meta-value) {
    color: rgba(255, 255, 255, 0.7);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
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
