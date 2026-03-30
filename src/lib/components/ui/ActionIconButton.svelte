<script lang="ts">
  import type { ComponentType, SvelteComponent } from "svelte";
  import type { IconProps } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  type LucideIcon = ComponentType<SvelteComponent<IconProps>>;

  let {
    icon,
    confirmIcon,
    onclick,
    title = "",
    disabled = false,
    size = ICON_SIZE.md,
    confirmed = $bindable(false),
  }: {
    icon: LucideIcon;
    confirmIcon?: LucideIcon;
    onclick: (e: MouseEvent) => void;
    title?: string;
    disabled?: boolean;
    size?: number;
    confirmed?: boolean;
  } = $props();
  let ActiveIcon = $derived(confirmed && confirmIcon ? confirmIcon : icon);

  function handleClick(e: MouseEvent) {
    e.stopPropagation();
    onclick(e);
    if (confirmIcon) {
      confirmed = true;
      setTimeout(() => (confirmed = false), 1200);
    }
  }
</script>

<button class="action-icon-btn" {title} {disabled} onclick={handleClick}>
  <ActiveIcon {size} />
</button>

<style>
  .action-icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
  }

  .action-icon-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.8);
  }

  .action-icon-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
