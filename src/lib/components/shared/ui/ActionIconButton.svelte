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
    active = false,
    variant = "default",
  }: {
    icon: LucideIcon;
    confirmIcon?: LucideIcon;
    onclick: (e: MouseEvent) => void;
    title?: string;
    disabled?: boolean;
    size?: number;
    confirmed?: boolean;
    active?: boolean;
    variant?: "default" | "danger";
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

<button class="action-icon-btn" class:active class:danger={variant === "danger"} {title} {disabled} onclick={handleClick}>
  <ActiveIcon {size} />
</button>

<style>
  .action-icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-2);
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .action-icon-btn:hover:not(:disabled) {
    background: var(--surface-overlay);
    color: var(--text-secondary);
  }

  .action-icon-btn.active,
  .action-icon-btn.active:hover:not(:disabled) {
    background: rgba(74, 158, 187, 0.2);
    color: var(--info);
  }

  .action-icon-btn.danger:hover:not(:disabled) {
    background: var(--danger-border);
    color: var(--danger);
  }

  .action-icon-btn:disabled {
    opacity: var(--opacity-disabled);
    cursor: default;
  }
</style>
