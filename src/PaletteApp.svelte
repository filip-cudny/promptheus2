<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { RefreshCw } from "lucide-svelte";
  import { providerIconSvg } from "$lib/icons/providerIcons";
  import { PROMPTHEUS_PROVIDER_ID, closePalette, reloadActiveInHost } from "$lib/services/shellToolbar";

  type ProviderEntry = { kind: "provider"; id: string; name: string; url?: string };
  type ActionEntry = { kind: "action"; id: string; name: string };
  type PaletteEntry = ProviderEntry | ActionEntry;

  type ShowPayload = {
    host_label: string;
    active_id: string;
    providers: { id: string; name: string; url?: string | null }[];
  };

  const ACTION_RELOAD_ID = "action:reload-active";

  let hostLabel = $state("");
  let activeId = $state(PROMPTHEUS_PROVIDER_ID);
  let webviewProviders = $state<{ id: string; name: string; url?: string | null }[]>([]);
  let query = $state("");
  let index = $state(0);
  let inputEl: HTMLInputElement | undefined = $state();
  let visible = $state(false);

  let providers = $derived<ProviderEntry[]>([
    { kind: "provider", id: PROMPTHEUS_PROVIDER_ID, name: "Promptheus" },
    ...webviewProviders.map<ProviderEntry>((p) => ({
      kind: "provider",
      id: p.id,
      name: p.name,
      url: p.url ?? undefined,
    })),
  ]);

  let activeName = $derived(providers.find((p) => p.id === activeId)?.name ?? "active provider");

  let actions = $derived<ActionEntry[]>([
    { kind: "action", id: ACTION_RELOAD_ID, name: `Reload ${activeName}` },
  ]);

  let entries = $derived<PaletteEntry[]>([...providers, ...actions]);

  let filtered = $derived.by<PaletteEntry[]>(() => {
    const q = query.trim().toLowerCase();
    if (!q) return entries;
    return entries.filter((e) => e.name.toLowerCase().includes(q));
  });

  $effect(() => {
    const _ = filtered.length;
    if (index >= filtered.length) {
      index = Math.max(0, filtered.length - 1);
    }
  });

  let unlistenShow: UnlistenFn | undefined;
  let unlistenBlur: UnlistenFn | undefined;

  async function dismiss(selectedId: string | null) {
    if (!hostLabel) return;
    visible = false;
    try {
      await closePalette(hostLabel, selectedId);
    } catch (e) {
      console.error("close_palette failed", e);
    }
  }

  async function reloadActive() {
    if (!hostLabel) return;
    try {
      await reloadActiveInHost(hostLabel);
    } catch (e) {
      console.error("reload_active_in_host failed", e);
    }
    await dismiss(null);
  }

  async function selectEntry(entry: PaletteEntry) {
    if (entry.kind === "action") {
      if (entry.id === ACTION_RELOAD_ID) {
        await reloadActive();
      }
      return;
    }
    await dismiss(entry.id);
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "r") {
      e.preventDefault();
      reloadActive();
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      dismiss(null);
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      const entry = filtered[index];
      if (entry) selectEntry(entry);
      return;
    }
    if (e.key === "ArrowDown" || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "j")) {
      e.preventDefault();
      index = Math.min(filtered.length - 1, index + 1);
      return;
    }
    if (e.key === "ArrowUp" || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k")) {
      e.preventDefault();
      index = Math.max(0, index - 1);
      return;
    }
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "p") {
      e.preventDefault();
      dismiss(null);
      return;
    }
  }

  onMount(async () => {
    window.addEventListener("keydown", handleKeydown);

    unlistenShow = await listen<ShowPayload>("palette:show", async (ev) => {
      hostLabel = ev.payload.host_label;
      activeId = ev.payload.active_id;
      webviewProviders = ev.payload.providers;
      query = "";
      index = 0;
      visible = false;
      await tick();
      visible = true;
      await tick();
      inputEl?.focus();
    });

    unlistenBlur = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (!focused) dismiss(null);
    });
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
    unlistenShow?.();
    unlistenBlur?.();
  });
</script>

{#if visible}
<button
  type="button"
  aria-label="Close palette"
  class="scrim"
  onclick={() => dismiss(null)}
></button>

<div class="modal" role="dialog" aria-modal="true">
  <input
    bind:this={inputEl}
    bind:value={query}
    oninput={() => (index = 0)}
    class="input"
    type="text"
    placeholder="Search providers and actions..."
    autocomplete="off"
    spellcheck="false"
  />
  <div class="list" role="listbox">
    {#each filtered as entry, i (entry.id)}
      {#if entry.kind === "action" && i > 0 && filtered[i - 1].kind === "provider"}
        <div class="divider" role="separator"></div>
      {/if}
      <button
        type="button"
        role="option"
        aria-selected={i === index}
        class="item"
        class:highlight={i === index}
        onmouseenter={() => (index = i)}
        onclick={() => selectEntry(entry)}
      >
        <span class="icon" class:muted={entry.kind === "action"} aria-hidden="true">
          {#if entry.kind === "provider"}
            {@const iconSvg = providerIconSvg(entry)}
            {#if iconSvg}
              {@html iconSvg}
            {/if}
          {:else}
            <RefreshCw size={14} />
          {/if}
        </span>
        <span class="name">{entry.name}</span>
        {#if entry.kind === "provider" && entry.id === activeId}
          <span class="badge">active</span>
        {/if}
      </button>
    {:else}
      <div class="empty">no matches</div>
    {/each}
  </div>
  <div class="footer">
    <span>↑↓ / ⌃jk navigate</span>
    <span>↵ select</span>
    <span>⌘R reload</span>
    <span>esc close</span>
  </div>
</div>
{/if}

<style>
  :global(html),
  :global(body) {
    background: transparent;
    margin: 0;
  }

  .scrim {
    position: fixed;
    inset: 0;
    background: transparent;
    border: 0;
    padding: 0;
    cursor: default;
  }

  .modal {
    position: fixed;
    top: 80px;
    left: 50%;
    transform: translateX(-50%);
    width: min(640px, 86%);
    background: #252525;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    color: #e0e0e0;
    font-size: 13px;
    animation: palette-modal-enter 140ms ease-out both;
  }

  :global([data-platform="linux"]) .modal {
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.22);
  }

  @keyframes palette-modal-enter {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .input {
    appearance: none;
    border: 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    background: transparent;
    color: #fff;
    font: inherit;
    font-size: 14px;
    padding: 12px 14px;
    outline: none;
  }

  .list {
    display: flex;
    flex-direction: column;
    max-height: 320px;
    overflow-y: auto;
  }

  .item {
    appearance: none;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.85);
    font: inherit;
    text-align: left;
    padding: 8px 14px;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .item.highlight {
    background: rgba(255, 255, 255, 0.08);
  }

  .icon {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.85);
  }

  .icon :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
  }

  .icon :global(img) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: contain;
  }

  .icon.muted {
    opacity: 0.45;
  }

  .item.highlight .icon.muted {
    opacity: 0.75;
  }

  .name {
    flex: 1;
    font-size: 13px;
  }

  .badge {
    font-size: 10px;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.45);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 3px;
    padding: 1px 5px;
  }

  .empty {
    color: rgba(255, 255, 255, 0.4);
    padding: 16px;
    text-align: center;
  }

  .divider {
    height: 1px;
    background: rgba(255, 255, 255, 0.06);
    margin: 4px 0;
  }

  .footer {
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    padding: 6px 14px;
    display: flex;
    gap: 12px;
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
  }
</style>
