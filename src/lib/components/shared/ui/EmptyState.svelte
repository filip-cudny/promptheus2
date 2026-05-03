<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    icon,
    title,
    description,
    action,
  }: {
    icon?: Snippet;
    title: string;
    description?: string;
    action?: Snippet;
  } = $props();
</script>

<div class="empty-state">
  {#if icon}
    <div class="empty-state-icon">{@render icon()}</div>
  {/if}
  <p class="empty-state-title">{title}</p>
  {#if description}
    <p class="empty-state-description">{description}</p>
  {/if}
  {#if action}
    <div class="empty-state-action">{@render action()}</div>
  {/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-12);
    text-align: center;
  }

  .empty-state-icon {
    color: var(--text-muted);
    opacity: 0.6;
  }

  .empty-state-title {
    font-size: var(--font-size-base);
    color: var(--text-secondary);
    font-weight: var(--font-weight-medium);
  }

  .empty-state-description {
    font-size: var(--font-size-md);
    color: var(--text-muted);
    max-width: 280px;
    line-height: var(--line-height-normal);
  }

  .empty-state-action {
    margin-top: var(--space-2);
  }
</style>
