<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    children,
    variant = "assistant",
  }: {
    children: Snippet;
    variant?: "assistant" | "user";
  } = $props();
</script>

<div class="bubble-edit-field" class:user={variant === "user"}>
  {@render children()}
</div>

<style>
  .bubble-edit-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    padding: var(--space-4) var(--space-4) var(--space-0);
  }

  .bubble-edit-field:focus-within {
    border-color: var(--border-strong);
  }

  .bubble-edit-field.user:focus-within {
    border-color: rgba(74, 158, 187, 0.4);
  }
</style>
