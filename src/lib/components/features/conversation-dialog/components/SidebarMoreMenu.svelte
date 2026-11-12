<script lang="ts">
  import { EllipsisVertical, Pencil, Trash2 } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";
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

<span class="more-wrap">
  <ActionIconButton icon={EllipsisVertical} label="More actions" bind:element={buttonEl} onclick={onToggle} />
</span>
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
  .more-wrap {
    flex-shrink: 0;
    display: flex;
    opacity: 0;
  }

  :global(.tab-item:hover) .more-wrap,
  .more-wrap:focus-within {
    opacity: 1;
  }
</style>
