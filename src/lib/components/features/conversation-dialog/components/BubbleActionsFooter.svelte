<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    visible = false,
    children,
  }: {
    visible?: boolean;
    children: Snippet;
  } = $props();
</script>

<div class="bubble-footer" class:actions-visible={visible}>
  {@render children()}
</div>

<style>
  .bubble-footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-0);
    opacity: 0;
    transition: opacity var(--motion-fast) var(--ease-default);
  }

  .bubble-footer.actions-visible,
  :global(.assistant-message-wrapper:hover) .bubble-footer,
  :global(.user-message-wrapper:hover) .bubble-footer {
    opacity: 1;
  }
</style>
