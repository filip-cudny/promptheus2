<script lang="ts">
  import { X } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    title = "Conversations",
    onClose,
  }: {
    title?: string;
    onClose: () => void;
  } = $props();
</script>

<div class="sidebar-header">
  <span class="sidebar-title">{title}</span>
  <button class="close-btn" onclick={onClose}>
    <X size={ICON_SIZE.md} />
  </button>
</div>

<style>
  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-6) var(--space-6) var(--space-4);
    border-bottom: 1px solid var(--border-default);
  }

  .close-btn {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-md);
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-0);
  }

  .close-btn:hover {
    color: var(--text-primary);
    background: var(--surface-overlay);
  }

  .sidebar-title {
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-semibold);
    color: var(--text-muted);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }
</style>
