<script lang="ts">
  import { EllipsisVertical, Pencil, Trash2 } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import FloatingPanel from "$lib/components/shared/ui/FloatingPanel.svelte";
  import MenuList from "$lib/components/shared/ui/MenuList.svelte";
  import MenuItem from "$lib/components/shared/ui/MenuItem.svelte";

  let {
    open,
    onToggle,
    onClose,
    onRename,
    onDelete,
  }: {
    open: boolean;
    onToggle: (e: MouseEvent) => void;
    onClose: () => void;
    onRename: () => void;
    onDelete: () => void;
  } = $props();

  let buttonEl: HTMLButtonElement | undefined = $state();
</script>

<button class="more-btn" bind:this={buttonEl} onclick={onToggle}>
  <EllipsisVertical size={14} />
</button>
<FloatingPanel visible={open} anchorEl={buttonEl} align="end" fitContent onclose={onClose}>
  <MenuList role="menu">
    <MenuItem
      label="Rename"
      onclick={(e) => {
        e.stopPropagation();
        onRename();
      }}
    >
      {#snippet icon()}<Pencil size={ICON_SIZE.md} />{/snippet}
    </MenuItem>
    <MenuItem
      label="Delete"
      onclick={(e) => {
        e.stopPropagation();
        onDelete();
      }}
    >
      {#snippet icon()}<Trash2 size={ICON_SIZE.md} />{/snippet}
    </MenuItem>
  </MenuList>
</FloatingPanel>

<style>
  .more-btn {
    flex-shrink: 0;
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
</style>
