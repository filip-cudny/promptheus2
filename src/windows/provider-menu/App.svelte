<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ProviderMenuList from "$lib/components/provider-menu/ProviderMenuList.svelte";

  type Provider = { id: string; name: string; url?: string | null };
  type ShowPayload = {
    providers: Provider[];
    active_id: string;
  };

  let providers = $state<Provider[]>([]);
  let activeId = $state<string>("");
  let unlistenShow: UnlistenFn | undefined;
  let unlistenBlur: UnlistenFn | undefined;

  async function pick(id: string) {
    try {
      await invoke("provider_menu_select", { providerId: id });
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
      providers = ev.payload.providers;
      activeId = ev.payload.active_id;
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
  <ProviderMenuList {providers} {activeId} onSelect={pick} />
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
    margin: var(--space-0);
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
