<script lang="ts">
  import { ChevronLeft, ChevronRight } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    current,
    total,
    onPrev,
    onNext,
  }: {
    current: number;
    total: number;
    onPrev: () => void;
    onNext: () => void;
  } = $props();
</script>

<span class="branch-nav">
  <button class="branch-btn" onclick={onPrev} disabled={current <= 1}>
    <ChevronLeft size={ICON_SIZE.md} />
  </button>
  <span class="branch-counter">{current}/{total}</span>
  <button class="branch-btn" onclick={onNext} disabled={current >= total}>
    <ChevronRight size={ICON_SIZE.md} />
  </button>
</span>

<style>
  .branch-nav {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .branch-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--text-muted);
    border-radius: var(--radius-md);
    padding: var(--space-1);
    cursor: pointer;
  }

  .branch-btn:hover:not(:disabled) {
    background: var(--surface-overlay);
    color: var(--text-secondary);
  }

  .branch-btn:disabled {
    opacity: var(--opacity-disabled);
    cursor: default;
  }

  .branch-counter {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    min-width: 24px;
    text-align: center;
  }
</style>
