<script lang="ts">
  import MenuList from "$lib/components/ui/MenuList.svelte";
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
    <button
      type="button"
      role="option"
      aria-selected={activeId === p.id}
      class="menu-list-item"
      class:is-active={activeId === p.id}
      onclick={() => onSelect(p.id)}
    >
      {#if iconSvg}
        <span class="menu-list-icon" aria-hidden="true">{@html iconSvg}</span>
      {:else}
        <span class="menu-list-icon" aria-hidden="true"></span>
      {/if}
      <span class="menu-list-label">{p.name}</span>
    </button>
  {/each}
</MenuList>
