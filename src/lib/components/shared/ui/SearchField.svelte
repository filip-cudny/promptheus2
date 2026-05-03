<script lang="ts">
  import { Search, X } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    value = $bindable(""),
    placeholder = "Search…",
    oninput,
    onchange,
  }: {
    value?: string;
    placeholder?: string;
    oninput?: (e: Event) => void;
    onchange?: (e: Event) => void;
  } = $props();

  function clear() {
    value = "";
  }
</script>

<div class="search-field">
  <Search size={ICON_SIZE.sm} class="search-icon" />
  <input
    class="search-input"
    type="text"
    bind:value
    {placeholder}
    {oninput}
    {onchange}
  />
  {#if value}
    <button class="clear-btn" onclick={clear} aria-label="Clear search">
      <X size={ICON_SIZE.sm} />
    </button>
  {/if}
</div>

<style>
  .search-field {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    background: var(--surface-elevated);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    padding: 0 var(--space-3);
    transition: border-color var(--motion-fast) var(--ease-default);
  }

  .search-field:focus-within {
    border-color: var(--accent-border);
  }

  .search-field :global(.search-icon) {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    padding: var(--space-2) 0;
    font-family: var(--font-sans);
    font-size: var(--font-size-md);
    color: var(--text-primary);
    min-width: 0;
  }

  .search-input::placeholder {
    color: var(--text-muted);
  }

  .clear-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    padding: var(--space-1);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    flex-shrink: 0;
  }

  .clear-btn:hover {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }
</style>
