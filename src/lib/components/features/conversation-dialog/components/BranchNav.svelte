<script lang="ts">
  import { ChevronLeft, ChevronRight } from "lucide-svelte";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";

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
  <ActionIconButton icon={ChevronLeft} label="Previous branch" onclick={onPrev} disabled={current <= 1} />
  <span class="branch-counter">{current}/{total}</span>
  <ActionIconButton icon={ChevronRight} label="Next branch" onclick={onNext} disabled={current >= total} />
</span>

<style>
  .branch-nav {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .branch-counter {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    min-width: 24px;
    text-align: center;
  }
</style>
