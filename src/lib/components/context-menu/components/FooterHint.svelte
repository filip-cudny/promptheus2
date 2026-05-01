<script lang="ts">
  import { ArrowBigUp } from "lucide-svelte";

  let { shiftHeld }: { shiftHeld: boolean } = $props();
</script>

<div class="footer-hint" class:active={shiftHeld}>
  <span class="footer-hint-key"><ArrowBigUp size={12} strokeWidth={2.25} /></span>
  <span>voice input</span>
  <span class="footer-hint-sep">·</span>
  <span>right-click for actions</span>
</div>

<style>
  .footer-hint {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-6) var(--space-1);
    margin-top: var(--space-1);
    font-size: var(--font-size-sm);
    color: var(--text-faint);
    user-select: none;
    transition: color var(--motion-fast) var(--ease-default);
  }

  .footer-hint.active {
    color: var(--text-muted);
  }

  .footer-hint-key {
    display: inline-flex;
    align-items: center;
  }

  .footer-hint-sep {
    color: rgba(255, 255, 255, 0.18);
  }
</style>
