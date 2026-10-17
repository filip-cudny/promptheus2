<script lang="ts">
  import type { Snippet } from "svelte";
  import { CopyPlus, Download, Trash2 } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import Button from "$lib/components/shared/ui/Button.svelte";

  let {
    noun,
    name,
    compact = false,
    warning,
    onDuplicate,
    onExport,
    onDelete,
  }: {
    noun: string;
    name: string;
    compact?: boolean;
    warning?: Snippet;
    onDuplicate: () => void;
    onExport?: () => void;
    onDelete: () => void;
  } = $props();

  let confirmDelete = $state(false);
</script>

<section class="card danger" class:compact>
  <h3>Danger zone</h3>
  <div class="danger-row">
    <button class="action-btn" onclick={onDuplicate}>
      <CopyPlus size={ICON_SIZE.md} />
      <span>Duplicate {noun}</span>
    </button>
    {#if onExport}
      <button class="action-btn" onclick={onExport}>
        <Download size={ICON_SIZE.md} />
        <span>Export .md</span>
      </button>
    {/if}
    {#if !confirmDelete}
      <button class="action-btn delete-btn" onclick={() => (confirmDelete = true)}>
        <Trash2 size={ICON_SIZE.md} />
        <span>Delete {noun}</span>
      </button>
    {:else}
      <div class="delete-confirm">
        <p>
          Delete <strong>{name}</strong>?
          {#if warning}
            <br />
            <span class="warn">{@render warning()}</span>
          {/if}
        </p>
        <div class="confirm-actions">
          <button class="cancel-btn" onclick={() => (confirmDelete = false)}>Cancel</button>
          <Button variant="danger" onclick={onDelete}>Yes, delete</Button>
        </div>
      </div>
    {/if}
  </div>
</section>

<style>
  .card {
    --danger-card-padding: var(--space-7) var(--space-8);
    --danger-card-gap: var(--space-6);
    --danger-row-gap: var(--space-5);
    --danger-confirm-gap: var(--space-4);
    background: var(--surface-base);
    border: 1px solid var(--danger-border);
    border-radius: var(--radius-lg);
    padding: var(--danger-card-padding);
    display: flex;
    flex-direction: column;
    gap: var(--danger-card-gap);
  }

  .card.compact {
    --danger-card-padding: var(--space-6) var(--space-7);
    --danger-card-gap: var(--space-5);
    --danger-row-gap: var(--space-4);
    --danger-confirm-gap: var(--space-3);
  }

  h3 {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--text-muted);
    margin: var(--space-0);
  }

  .danger-row {
    display: flex;
    flex-direction: column;
    gap: var(--danger-row-gap);
  }

  .action-btn {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-6);
    background: transparent;
    border: 1px solid var(--border-hard);
    border-radius: 5px;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
  }

  .action-btn:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .delete-btn {
    color: var(--danger);
    border-color: var(--danger-border);
  }

  .delete-btn:hover {
    background: var(--danger-bg-soft);
  }

  .delete-confirm p {
    font-size: var(--font-size-md);
    color: var(--text-primary);
    margin: var(--space-0) var(--space-0) var(--danger-confirm-gap);
  }

  .delete-confirm .warn {
    color: var(--warning);
    font-size: var(--font-size-sm);
  }

  .confirm-actions {
    display: flex;
    gap: var(--danger-confirm-gap);
  }

  .cancel-btn {
    padding: 5px var(--space-5);
    background: var(--surface-elevated);
    border: 1px solid var(--border-hard-2);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
  }
</style>
