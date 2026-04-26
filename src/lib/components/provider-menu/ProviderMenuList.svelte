<script lang="ts">
  import { providerIconSvg } from "$lib/icons/providerIcons";

  type Provider = { id: string; name: string; url?: string | null };

  let {
    providers,
    activeId = "",
    onSelect,
  }: {
    providers: Provider[];
    activeId?: string;
    onSelect: (id: string) => void;
  } = $props();
</script>

<div class="menu" role="listbox">
  {#each providers as p (p.id)}
    {@const iconSvg = providerIconSvg(p)}
    <button
      type="button"
      role="option"
      aria-selected={activeId === p.id}
      class="item"
      class:active={activeId === p.id}
      onclick={() => onSelect(p.id)}
    >
      {#if iconSvg}
        <span class="favicon" aria-hidden="true">{@html iconSvg}</span>
      {:else}
        <span class="favicon favicon-placeholder" aria-hidden="true"></span>
      {/if}
      <span class="label">{p.name}</span>
    </button>
  {/each}
</div>

<style>
  .menu {
    display: inline-flex;
    flex-direction: column;
    min-width: 160px;
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 12px;
    overflow: hidden;
  }

  .item {
    appearance: none;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.75);
    padding: 6px 12px;
    text-align: left;
    cursor: pointer;
    font: inherit;
    line-height: 1;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .favicon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: #fff;
  }

  .favicon :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
  }

  .favicon :global(img) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: contain;
  }

  .favicon-placeholder {
    background: transparent;
  }

  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }

  .item.active {
    color: #fff;
    background: rgba(255, 255, 255, 0.06);
  }
</style>
