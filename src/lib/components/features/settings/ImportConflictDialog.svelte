<script lang="ts">
  import Modal from "$lib/components/shared/ui/Modal.svelte";
  import type { ImportConflictMode } from "$lib/types";

  let {
    existingSlug,
    onClose,
    onResolve,
  }: {
    existingSlug: string;
    onClose: () => void;
    onResolve: (mode: ImportConflictMode) => void;
  } = $props();
</script>

<Modal title="Skill already exists" width={520} {onClose}>
  <p>
    A skill named <code>/{existingSlug}</code> already exists. Choose how to import:
  </p>

  <footer>
    <button class="cancel" onclick={onClose}>Cancel</button>
    <button
      class="rename"
      onclick={() => {
        onResolve("rename_suffix");
        onClose();
      }}
    >
      Import as <code>{existingSlug}-imported</code>
    </button>
    <button
      class="overwrite"
      onclick={() => {
        onResolve("overwrite");
        onClose();
      }}
    >
      Overwrite
    </button>
  </footer>
</Modal>

<style>
  p {
    margin: 0;
    font-size: var(--font-size-md);
    color: var(--text-secondary);
  }

  code {
    font-family: var(--font-mono);
    padding: 0 4px;
    background: var(--surface-overlay-faint);
    border-radius: var(--radius-sm);
    color: var(--accent);
  }

  footer {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-3);
    padding-top: var(--space-2);
  }

  footer button {
    padding: var(--space-2) var(--space-5);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-hard-2);
    background: var(--surface-elevated);
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .rename {
    border-color: var(--accent-border);
    color: var(--accent);
  }

  .overwrite {
    border-color: var(--danger-border);
    color: var(--danger);
    background: var(--danger-bg-soft);
  }
</style>
