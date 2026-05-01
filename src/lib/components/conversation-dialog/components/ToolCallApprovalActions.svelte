<script lang="ts">
  import { Check, X } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    onApprove,
    onReject,
  }: {
    onApprove?: () => void;
    onReject?: () => void;
  } = $props();
</script>

{#if onApprove || onReject}
  <div class="confirmation-actions">
    {#if onApprove}
      <button class="approve-btn" onclick={onApprove}>
        <Check size={ICON_SIZE.sm} />
        Approve
      </button>
    {/if}
    {#if onReject}
      <button class="reject-btn" onclick={onReject}>
        <X size={ICON_SIZE.sm} />
        Reject
      </button>
    {/if}
  </div>
{/if}

<style>
  .confirmation-actions {
    display: flex;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-5) var(--space-4);
  }

  .approve-btn,
  .reject-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-5);
    border: none;
    border-radius: var(--radius-md);
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
  }

  .approve-btn {
    background: var(--success-bg-soft);
    color: var(--success);
  }

  .approve-btn:hover {
    background: var(--success-border);
  }

  .reject-btn {
    background: var(--danger-bg-soft);
    color: var(--text-muted);
  }

  .reject-btn:hover {
    background: var(--danger-border);
    color: var(--danger);
  }
</style>
