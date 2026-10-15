<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import MenuList from "$lib/components/shared/ui/MenuList.svelte";
  import MenuItem from "$lib/components/shared/ui/MenuItem.svelte";
  import { getWebviewProviders, type WebviewProvider } from "$lib/services/aiWebview";
  import { providerIconSvg } from "$lib/icons/providerIcons";
  import type { ProviderMenuItem } from "$lib/services/shellToolbar";

  type ShowPayload = { items: ProviderMenuItem[] };

  let items = $state<ProviderMenuItem[]>([]);
  let webviewProviders = $state<WebviewProvider[]>([]);
  let unlistenShow: UnlistenFn | undefined;
  let unlistenBlur: UnlistenFn | undefined;

  async function pick(id: string) {
    try {
      await invoke("provider_menu_select", { itemId: id });
    } catch (e) {
      console.error("provider_menu_select failed", e);
    }
  }

  async function close() {
    try {
      await invoke("hide_provider_menu");
    } catch (e) {
      console.error("hide_provider_menu failed", e);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  }

  async function reportSize() {
    await tick();
    const root = document.getElementById("menu-root");
    if (!root) return;
    const rect = root.getBoundingClientRect();
    try {
      await invoke("size_provider_menu", {
        width: Math.ceil(rect.width),
        height: Math.ceil(rect.height),
      });
    } catch (e) {
      console.error("size_provider_menu failed", e);
    }
  }

  onMount(async () => {
    window.addEventListener("keydown", handleKeydown);

    unlistenShow = await listen<ShowPayload>("provider-menu:show", async (ev) => {
      items = ev.payload.items;
      try {
        webviewProviders = await getWebviewProviders();
      } catch (e) {
        console.error("getWebviewProviders failed", e);
      }
      await reportSize();
    });

    unlistenBlur = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (!focused) close();
    });
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
    unlistenShow?.();
    unlistenBlur?.();
  });
</script>

<div id="menu-root" class="menu-shell">
  <MenuList>
    {#each items as item (item.id)}
      {@const iconSvg = providerIconSvg({
        id: item.icon,
        url: webviewProviders.find((p) => p.id === item.icon)?.url,
      })}
      {#snippet itemIcon()}
        <span class="item-icon" aria-hidden="true">{#if iconSvg}{@html iconSvg}{/if}</span>
      {/snippet}
      <MenuItem
        label={item.title}
        icon={itemIcon}
        active={item.active}
        activeStyle="rail"
        onclick={() => pick(item.id)}
      />
    {/each}
  </MenuList>
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
    margin: var(--space-0);
  }

  .item-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .item-icon :global(svg),
  .item-icon :global(img) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: contain;
  }

  .menu-shell {
    display: inline-flex;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    padding: var(--space-2) var(--space-0);
    overflow: hidden;
  }
</style>
