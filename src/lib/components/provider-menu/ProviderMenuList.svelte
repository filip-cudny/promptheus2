<script lang="ts">
  import MenuList from "$lib/components/ui/MenuList.svelte";
  import MenuItem from "$lib/components/ui/MenuItem.svelte";
  import { providerIconSvg } from "$lib/icons/providerIcons";

  type Provider = { id: string; name: string; url?: string | null };

  let {
    providers,
    activeId = "",
    expand = false,
    onSelect,
  }: {
    providers: Provider[];
    activeId?: string;
    expand?: boolean;
    onSelect: (id: string) => void;
  } = $props();
</script>

<MenuList {expand}>
  {#each providers as p (p.id)}
    {@const iconSvg = providerIconSvg(p)}
    {#snippet providerIcon()}
      {#if iconSvg}
        <span class="provider-icon" aria-hidden="true">{@html iconSvg}</span>
      {:else}
        <span class="provider-icon" aria-hidden="true"></span>
      {/if}
    {/snippet}
    <MenuItem
      label={p.name}
      icon={providerIcon}
      active={activeId === p.id}
      onclick={() => onSelect(p.id)}
    />
  {/each}
</MenuList>

<style>
  .provider-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .provider-icon :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
  }

  .provider-icon :global(img) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: contain;
  }
</style>
