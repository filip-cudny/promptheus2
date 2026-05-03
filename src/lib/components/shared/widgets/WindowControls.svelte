<script lang="ts">
  import { Minus, Square, X } from "lucide-svelte";

  let {
    isMaximized = false,
    onMinimize,
    onToggleMaximize,
    onClose,
  }: {
    isMaximized?: boolean;
    onMinimize: () => void;
    onToggleMaximize: () => void;
    onClose: () => void;
  } = $props();
</script>

<div class="actions">
  <button type="button" class="win-btn" title="Minimize" onclick={onMinimize}>
    <Minus size={14} />
  </button>
  <button type="button" class="win-btn" title={isMaximized ? "Restore" : "Maximize"} onclick={onToggleMaximize}>
    <Square size={12} />
  </button>
  <button type="button" class="win-btn close" title="Close" onclick={onClose}>
    <X size={14} />
  </button>
</div>

<style>
  .actions {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .win-btn {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--text-muted);
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
    cursor: pointer;
  }

  .win-btn:hover {
    color: var(--text-primary);
    background: var(--surface-overlay);
  }

  .win-btn.close:hover {
    background: var(--surface-overlay-strong);
    color: var(--text-primary);
  }
</style>
