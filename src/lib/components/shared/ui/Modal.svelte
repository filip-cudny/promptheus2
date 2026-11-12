<script lang="ts">
  import type { Snippet } from "svelte";
  import { X } from "lucide-svelte";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";

  let {
    title,
    width = 480,
    onClose,
    children,
  }: {
    title: string;
    width?: number;
    onClose: () => void;
    children: Snippet;
  } = $props();

  function handleBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={handleKey} />

<div class="backdrop" onmousedown={handleBackdrop} role="presentation">
  <div class="dialog" style="--modal-width: {width}px">
    <header>
      <h2>{title}</h2>
      <ActionIconButton icon={X} label="Close" size="lg" onclick={onClose} />
    </header>
    {@render children()}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--z-modal, 1000);
  }

  .dialog {
    width: var(--modal-width);
    max-width: calc(100vw - var(--space-8));
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    padding: var(--space-6) var(--space-7) var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    box-shadow: var(--shadow-lg);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  header h2 {
    margin: 0;
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
  }
</style>
