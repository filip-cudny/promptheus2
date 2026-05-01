<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    visible = false,
    align = "end",
    children,
  }: {
    visible?: boolean;
    align?: "start" | "end";
    children: Snippet;
  } = $props();
</script>

<div class="bubble-footer" class:actions-visible={visible} class:align-start={align === "start"}>
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

  .bubble-footer.align-start {
    justify-content: flex-start;
  }

  .bubble-footer.actions-visible,
  :global(.assistant-message-wrapper:hover) .bubble-footer,
  :global(.user-message-wrapper:hover) .bubble-footer {
    opacity: 1;
  }
</style>
