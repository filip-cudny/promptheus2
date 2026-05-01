<script lang="ts">
  let { message = "No items available" }: { message?: string } = $props();
</script>

<div class="empty-state" role="menuitem">{message}</div>

<style>
  .empty-state {
    padding: var(--space-6) var(--space-8);
    color: var(--text-disabled);
    text-align: center;
    font-style: italic;
  }
</style>
