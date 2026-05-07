<script lang="ts" generics="T extends string">
  let {
    tabs,
    active = $bindable<T>(),
  }: {
    tabs: ReadonlyArray<{ id: T; label: string }>;
    active: T;
  } = $props();
</script>

<div class="tab-strip" role="tablist">
  {#each tabs as t (t.id)}
    <button
      type="button"
      role="tab"
      aria-selected={active === t.id}
      class="tab"
      class:active={active === t.id}
      onclick={() => (active = t.id)}
    >
      {t.label}
    </button>
  {/each}
</div>

<style>
  .tab-strip {
    display: flex;
    align-items: stretch;
    gap: var(--space-1);
    border-bottom: 1px solid var(--border-faint);
  }

  .tab {
    appearance: none;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    padding: var(--space-2) var(--space-4);
    margin-bottom: -1px;
    font: inherit;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    color: var(--text-muted);
    cursor: pointer;
    transition: color var(--motion-fast) var(--ease-default),
      border-color var(--motion-fast) var(--ease-default),
      background var(--motion-fast) var(--ease-default);
  }

  .tab:hover:not(.active) {
    color: var(--text-primary);
    background: var(--surface-overlay-faint);
  }

  .tab.active {
    color: var(--text-primary);
    border-bottom-color: var(--accent);
  }

  .tab:focus-visible {
    outline: none;
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
  }
</style>
