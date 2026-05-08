<script lang="ts">
  import { X } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
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

  function handleBackdrop(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains("backdrop")) {
      onClose();
    }
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={handleKey} />

<div class="backdrop" onmousedown={handleBackdrop} role="presentation">
  <div class="dialog">
    <header>
      <h2>Skill already exists</h2>
      <button class="close" onclick={onClose} aria-label="Close">
        <X size={ICON_SIZE.md} />
      </button>
    </header>

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
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--z-modal, 1000);
  }

  .dialog {
    width: 520px;
    max-width: calc(100vw - var(--space-8));
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    padding: var(--space-6) var(--space-7) var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    box-shadow: var(--shadow-lg);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  header h2 {
    margin: 0;
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
  }

  .close {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: var(--space-1);
    border-radius: var(--radius-sm);
  }

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
