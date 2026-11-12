<script lang="ts">
  import type { Snippet } from "svelte";
  import { CopyPlus, Download, Trash2 } from "lucide-svelte";
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
    <div>
      <Button icon={CopyPlus} onclick={onDuplicate}>Duplicate {noun}</Button>
    </div>
    {#if onExport}
      <div>
        <Button icon={Download} onclick={onExport}>Export .md</Button>
      </div>
    {/if}
    {#if !confirmDelete}
      <div>
        <Button variant="danger" icon={Trash2} onclick={() => (confirmDelete = true)}>Delete {noun}</Button>
      </div>
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
          <Button onclick={() => (confirmDelete = false)}>Cancel</Button>
          <Button variant="danger" onclick={onDelete}>Delete {noun}</Button>
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
</style>
