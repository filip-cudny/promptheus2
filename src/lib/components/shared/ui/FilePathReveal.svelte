<script lang="ts">
  let { path }: { path: string } = $props();
</script>

<details class="path-details">
  <summary title="Show file path">…</summary>
  <code class="path">{path}</code>
</details>

<style>
  .path-details {
    position: relative;
    font-size: var(--font-size-xs);
  }

  .path-details summary {
    list-style: none;
    cursor: pointer;
    color: var(--text-faint);
    padding: 0 var(--space-2);
    border-radius: var(--radius-sm);
    user-select: none;
  }

  .path-details summary::-webkit-details-marker {
    display: none;
  }

  .path-details summary:hover {
    color: var(--text-muted);
    background: var(--surface-overlay-faint);
  }

  .path-details[open] summary {
    color: var(--text-muted);
  }

  .path-details code {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: var(--space-1);
    padding: var(--space-2) var(--space-3);
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--text-muted);
    background: var(--surface-floating-popover);
    border: 1px solid var(--surface-floating-popover-border);
    border-radius: var(--radius-sm);
    white-space: nowrap;
    box-shadow: var(--shadow-md);
    z-index: var(--z-overlay);
  }
</style>
