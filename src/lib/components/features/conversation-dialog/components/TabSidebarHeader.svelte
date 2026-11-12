<script lang="ts">
  import { X } from "lucide-svelte";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";

  let {
    title = "History",
    onClose,
  }: {
    title?: string;
    onClose: () => void;
  } = $props();
</script>

<div class="sidebar-header">
  <span class="sidebar-title">{title}</span>
  <ActionIconButton icon={X} size="lg" label="Close conversation list" onclick={onClose} />
</div>

<style>
  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-6) var(--space-6) var(--space-4);
    border-bottom: 1px solid var(--border-default);
  }

  .sidebar-title {
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-semibold);
    color: var(--text-muted);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }
</style>
