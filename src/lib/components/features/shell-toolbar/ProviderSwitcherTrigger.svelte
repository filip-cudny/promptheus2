<script lang="ts">
  import { ChevronDown } from "lucide-svelte";
  import { providerIconSvg } from "$lib/icons/providerIcons";

  type Provider = { id: string; name: string; url?: string };

  let {
    activeProvider,
    expanded = false,
    triggerEl = $bindable(null),
    onToggle,
  }: {
    activeProvider: Provider | undefined;
    expanded?: boolean;
    triggerEl?: HTMLButtonElement | null;
    onToggle: (e: MouseEvent) => void;
  } = $props();

  let iconSvg = $derived(providerIconSvg(activeProvider));
</script>

<button
  bind:this={triggerEl}
  type="button"
  class="trigger"
  aria-haspopup="listbox"
  aria-expanded={expanded}
  onmousedown={onToggle}
>
  {#if iconSvg}
    <span class="trigger-icon" aria-hidden="true">{@html iconSvg}</span>
  {/if}
  <span class="trigger-label">{activeProvider?.name ?? "Promptheus"}</span>
  <ChevronDown size={14} />
</button>

<style>
  .trigger {
    appearance: none;
    border: 1px solid var(--border-default);
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
    padding: 7px var(--space-3) 7px var(--space-4);
    border-radius: var(--radius-lg);
    font: inherit;
    cursor: pointer;
    line-height: 1;
    display: inline-flex;
    align-items: center;
    gap: var(--space-5);
    min-width: 110px;
  }

  .trigger:hover {
    background: var(--surface-overlay);
  }

  .trigger-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-primary);
  }

  .trigger-icon :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
    transform: scale(1.5);
    transform-origin: center;
  }

  .trigger-icon :global(img) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: contain;
    transform: scale(1.5);
    transform-origin: center;
  }

  .trigger-label {
    flex: 1;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .trigger :global(svg) {
    color: var(--text-muted);
    flex-shrink: 0;
  }
</style>
