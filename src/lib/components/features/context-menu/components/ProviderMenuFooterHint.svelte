<script lang="ts">
  import { ExternalLink } from "lucide-svelte";
</script>

<div class="provider-menu-footer-hint">
  <span class="provider-menu-footer-hint-icon">
    <ExternalLink size={12} strokeWidth={2.25} />
  </span>
  <span>Opens in a new window</span>
</div>

<style>
  .provider-menu-footer-hint {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4) var(--space-1);
    margin-top: var(--space-1);
    border-top: 1px solid var(--surface-overlay);
    font-size: var(--font-size-sm);
    color: var(--text-faint);
    user-select: none;
    white-space: nowrap;
  }

  .provider-menu-footer-hint-icon {
    display: inline-flex;
    align-items: center;
  }
</style>
