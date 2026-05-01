<script lang="ts">
  import { Wrench } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let { label }: { label: string } = $props();
</script>

<div class="tool-call-chip" title="Tool call (read-only)">
  <Wrench size={ICON_SIZE.sm} />
  <span>{label}</span>
</div>

<style>
  .tool-call-chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    margin: var(--space-1) var(--space-0);
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: var(--font-size-md);
    width: fit-content;
    user-select: none;
  }
</style>
