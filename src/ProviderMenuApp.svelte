<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  type Provider = { id: string; name: string };
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

<div id="menu-root" class="menu" role="listbox">
  {#each providers as p (p.id)}
    <button
      type="button"
      role="option"
      aria-selected={activeId === p.id}
      class="item"
      class:active={activeId === p.id}
      onclick={() => pick(p.id)}
    >
      {p.name}
    </button>
  {/each}
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
  }

  .menu {
    display: inline-flex;
    flex-direction: column;
    min-width: 160px;
    background: #252525;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    padding: 4px 0;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.5);
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
