<script lang="ts">
  import { ChevronRight, SlidersHorizontal } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";
  import MenuItemRow from "./MenuItemRow.svelte";

  type Props = {
    expanded: boolean;
    anchorEl?: HTMLElement;
    onclick: () => void;
    onhover: () => void;
    onOpenSettings: () => void;
  };

  let {
    expanded,
    anchorEl = $bindable(),
    onclick,
    onhover,
    onOpenSettings,
  }: Props = $props();
</script>

<MenuItemRow label="Settings" bind:rowEl={anchorEl} {onclick} {onhover}>
  {#snippet icon()}
    <span class="settings-chevron" class:expanded>
      <ChevronRight size={ICON_SIZE.sm} />
    </span>
  {/snippet}
  {#snippet trailing()}
    <ActionIconButton
      icon={SlidersHorizontal}
      size={ICON_SIZE.sm}
      title="Open settings window"
      onclick={onOpenSettings}
    />
  {/snippet}
</MenuItemRow>

<style>
  .settings-chevron {
    display: flex;
    align-items: center;
    margin-right: calc(-1 * var(--space-2));
    transition: transform var(--motion-default) var(--ease-default);
    color: var(--text-disabled);
  }

  .settings-chevron.expanded {
    transform: rotate(90deg);
  }
</style>
