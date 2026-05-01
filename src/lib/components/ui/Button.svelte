<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    variant = "ghost",
    disabled = false,
    active = false,
    type = "button",
    onclick,
    children,
    ...rest
  }: {
    variant?: "primary" | "ghost" | "danger" | "segment" | "chrome";
    disabled?: boolean;
    active?: boolean;
    type?: "button" | "submit" | "reset";
    onclick?: (e: MouseEvent) => void;
    children: Snippet;
    [key: string]: unknown;
  } = $props();
</script>

<button
  class="btn btn-{variant}"
  class:active
  {disabled}
  {type}
  {onclick}
  {...rest}
>
  {@render children()}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    border: none;
    border-radius: var(--radius-md);
    font-family: var(--font-sans);
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-medium);
    cursor: pointer;
    transition: background var(--motion-fast) var(--ease-default),
                color var(--motion-fast) var(--ease-default);
    white-space: nowrap;
    padding: var(--space-2) var(--space-4);
  }

  .btn:disabled {
    opacity: var(--opacity-disabled);
    cursor: default;
  }

  /* --- ghost (default) --- */
  .btn-ghost {
    background: transparent;
    color: var(--text-secondary);
  }
  .btn-ghost:hover:not(:disabled) {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }
  .btn-ghost.active {
    background: var(--surface-overlay-strong);
    color: var(--text-primary);
  }

  /* --- primary --- */
  .btn-primary {
    background: var(--accent);
    color: var(--accent-fg);
    padding: var(--space-2) var(--space-6);
  }
  .btn-primary:hover:not(:disabled) {
    background: var(--accent-bg);
    color: var(--accent-fg);
  }

  /* --- danger --- */
  .btn-danger {
    background: var(--danger-bg-soft);
    color: var(--danger);
    border: 1px solid var(--danger-border);
  }
  .btn-danger:hover:not(:disabled) {
    background: var(--danger-border);
    color: var(--danger);
  }

  /* --- segment --- */
  .btn-segment {
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    padding: var(--space-1) var(--space-3);
    font-size: var(--font-size-sm);
  }
  .btn-segment:hover:not(:disabled) {
    background: var(--surface-overlay);
    color: var(--text-secondary);
  }
  .btn-segment.active {
    background: var(--surface-elevated);
    color: var(--text-primary);
  }

  /* --- chrome (window chrome icon buttons) --- */
  .btn-chrome {
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-md);
    padding: var(--space-1);
    width: 26px;
    height: 26px;
  }
  .btn-chrome:hover:not(:disabled) {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }
</style>
