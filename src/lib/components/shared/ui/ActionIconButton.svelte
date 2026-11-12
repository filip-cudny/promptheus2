<script lang="ts">
  import type { ComponentType, SvelteComponent } from "svelte";
  import type { IconProps } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { useCopyFeedback } from "$lib/utils/copyFeedback.svelte";

  type LucideIcon = ComponentType<SvelteComponent<IconProps>>;

  let {
    icon,
    confirmIcon,
    onclick,
    label,
    title,
    disabled = false,
    size = "md",
    confirmed = false,
    active = false,
    variant = "default",
    element = $bindable(),
    ...rest
  }: {
    icon: LucideIcon;
    confirmIcon?: LucideIcon;
    onclick?: (e: MouseEvent) => void;
    label: string;
    title?: string;
    disabled?: boolean;
    size?: "md" | "lg";
    confirmed?: boolean;
    active?: boolean;
    variant?: "default" | "danger" | "primary";
    element?: HTMLButtonElement;
    [key: string]: unknown;
  } = $props();
  const feedback = useCopyFeedback<true>();
  let ActiveIcon = $derived(confirmIcon && (confirmed || feedback.confirmed) ? confirmIcon : icon);
  let iconSize = $derived(size === "lg" || variant === "primary" ? ICON_SIZE.lg : ICON_SIZE.md);

  function handleClick(e: MouseEvent) {
    e.stopPropagation();
    onclick?.(e);
    if (confirmIcon) feedback.flash(true);
  }
</script>

<button
  {...rest}
  bind:this={element}
  type="button"
  class="action-icon-btn"
  class:lg={size === "lg"}
  class:active
  class:danger={variant === "danger"}
  class:primary={variant === "primary"}
  aria-label={label}
  title={title ?? label}
  {disabled}
  onclick={handleClick}
>
  <ActiveIcon size={iconSize} strokeWidth={variant === "primary" ? 3 : undefined} />
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

  .action-icon-btn.lg {
    padding: var(--space-3);
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

  .action-icon-btn.primary,
  .action-icon-btn.primary:hover:not(:disabled) {
    width: 28px;
    height: 28px;
    padding: 0;
    border-radius: 50%;
    color: var(--surface-base);
  }

  .action-icon-btn.primary {
    background: var(--text-primary);
  }

  .action-icon-btn.primary:hover:not(:disabled) {
    background: var(--text-secondary);
  }

  .action-icon-btn:disabled {
    opacity: var(--opacity-disabled);
    cursor: default;
  }
</style>
