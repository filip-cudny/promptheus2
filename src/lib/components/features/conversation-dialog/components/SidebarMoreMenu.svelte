<script lang="ts">
  import { EllipsisVertical, Pencil, Trash2 } from "lucide-svelte";

  let {
    open,
    onToggle,
    onRename,
    onDelete,
    containerEl = $bindable(undefined),
  }: {
    open: boolean;
    onToggle: (e: MouseEvent) => void;
    onRename: () => void;
    onDelete: () => void;
    containerEl?: HTMLDivElement | undefined;
  } = $props();
</script>

<div class="more-menu" bind:this={containerEl}>
  <button class="more-btn" onclick={onToggle}>
    <EllipsisVertical size={14} />
  </button>
  {#if open}
    <div class="menu-dropdown">
      <button
        class="menu-item"
        onclick={(e: MouseEvent) => {
          e.stopPropagation();
          onRename();
        }}
      >
        <Pencil size={14} />
        <span>Rename</span>
      </button>
      <button
        class="menu-item destructive"
        onclick={(e: MouseEvent) => {
          e.stopPropagation();
          onDelete();
        }}
      >
        <Trash2 size={14} />
        <span>Delete</span>
      </button>
    </div>
  {/if}
</div>

<style>
  .more-menu {
    position: relative;
    flex-shrink: 0;
  }

  .more-btn {
    width: 20px;
    height: 20px;
    border-radius: var(--radius-md);
    border: none;
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-0);
    opacity: 0;
  }

  :global(.tab-item:hover) .more-btn,
  .more-btn:focus {
    opacity: 1;
  }

  .more-btn:hover {
    color: var(--text-primary);
    background: var(--surface-overlay);
  }

  .menu-dropdown {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: var(--space-1);
    min-width: 120px;
    background: var(--surface-floating-popover);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    padding: var(--space-2);
    z-index: 300;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4);
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: var(--font-size-md);
    cursor: pointer;
    border-radius: var(--radius-md);
    white-space: nowrap;
  }

  .menu-item:hover,
  .menu-item.destructive:hover {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }
</style>
