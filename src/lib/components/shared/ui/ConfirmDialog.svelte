<script lang="ts">
  import { Trash2 } from "lucide-svelte";
  import type { ComponentType, SvelteComponent } from "svelte";
  import type { IconProps } from "lucide-svelte";

  type LucideIcon = ComponentType<SvelteComponent<IconProps>>;

  let {
    open,
    message,
    confirmLabel = "Delete",
    confirmVariant = "danger",
    confirmIcon = Trash2,
    onConfirm,
    onCancel,
  }: {
    open: boolean;
    message: string;
    confirmLabel?: string;
    confirmVariant?: "danger" | "default";
    confirmIcon?: LucideIcon | null;
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      onConfirm();
    } else if (e.key === "Escape") {
      e.preventDefault();
      onCancel();
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="confirm-overlay" onclick={onCancel}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div
      class="confirm-dialog"
      onclick={(e: MouseEvent) => e.stopPropagation()}
      onkeydown={handleKeydown}
    >
      <p class="confirm-text">{message}</p>
      <div class="confirm-actions">
        <button class="confirm-btn cancel" onclick={onCancel}>Cancel</button>
        <!-- svelte-ignore a11y_autofocus -->
        <button
          class="confirm-btn"
          class:danger={confirmVariant === "danger"}
          onclick={onConfirm}
          autofocus
        >
          {#if confirmIcon}
            {@const Icon = confirmIcon}
            <Icon size={14} />
          {/if}
          <span>{confirmLabel}</span>
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .confirm-overlay {
    position: fixed;
    inset: 0;
    background: var(--surface-scrim);
    z-index: 400;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .confirm-dialog {
    background: var(--surface-sunken);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-xl);
    padding: var(--space-8) var(--space-10);
    min-width: 240px;
    display: flex;
    flex-direction: column;
    gap: var(--space-7);
  }

  .confirm-text {
    margin: var(--space-0);
    font-size: var(--font-size-base);
    color: var(--text-primary);
  }

  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-4);
  }

  .confirm-btn {
    padding: 5px var(--space-7);
    border-radius: 5px;
    border: none;
    font-size: var(--font-size-md);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .confirm-btn.cancel {
    background: var(--surface-overlay);
    color: var(--text-secondary);
  }

  .confirm-btn.cancel:hover {
    background: rgba(255, 255, 255, 0.14);
  }

  .confirm-btn.danger {
    background: var(--surface-overlay-strong);
    color: var(--text-primary);
    padding: 5px var(--space-5);
  }

  .confirm-btn.danger:hover {
    background: rgba(255, 255, 255, 0.14);
    color: var(--text-primary);
  }
</style>
