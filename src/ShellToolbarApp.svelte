<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { ChevronDown, Minus, Square, SquareArrowOutUpRight, X } from "lucide-svelte";
  import {
    getWebviewProviders,
    swapAiWebview,
    swapToConversationDialog,
    type WebviewProvider,
  } from "$lib/services/aiWebview";
  import { openConversationDialogNewWindow } from "$lib/services/conversationDialog";
  import { onSettingsChanged } from "$lib/services/events";
  import {
    PROMPTHEUS_PROVIDER_ID,
    getActiveProvider,
    hideProviderMenu,
    openPalette,
    showProviderMenu,
  } from "$lib/services/shellToolbar";
  import { providerIconSvg } from "$lib/icons/providerIcons";

  const HOST_LABEL = getCurrentWindow().label;
  const SELF_TARGET = getCurrentWebview().label;
  const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);
  const shortcutHint = isMac ? "⌘P" : "Ctrl P";

  let webviewProviders = $state<WebviewProvider[]>([]);
  let activeId = $state<string>(PROMPTHEUS_PROVIDER_ID);
  let isMaximized = $state(false);
  let providerDropdownOpen = $state(false);
  let triggerEl = $state<HTMLButtonElement | null>(null);

  let providers = $derived<{ id: string; name: string; url?: string }[]>([
    { id: PROMPTHEUS_PROVIDER_ID, name: "Promptheus" },
    ...webviewProviders.map((p) => ({ id: p.id, name: p.name, url: p.url })),
  ]);

  let activeProvider = $derived(
    providers.find((p) => p.id === activeId) ?? providers[0],
  );

  let activeIconSvg = $derived(providerIconSvg(activeProvider));

  let unlistenActive: UnlistenFn | undefined;
  let unlistenSelect: UnlistenFn | undefined;
  let unlistenClosed: UnlistenFn | undefined;
  let unlistenSettingsChanged: UnlistenFn | undefined;

  async function refreshWebviewProviders() {
    try {
      webviewProviders = await getWebviewProviders();
    } catch (e) {
      console.error("getWebviewProviders failed", e);
    }
  }

  async function refreshActive() {
    try {
      const pid = await getActiveProvider(HOST_LABEL);
      activeId = pid ?? PROMPTHEUS_PROVIDER_ID;
    } catch (e) {
      console.error("get_active_provider failed", e);
    }
  }

  async function selectProvider(id: string) {
    providerDropdownOpen = false;
    if (id === activeId) return;
    try {
      if (id === PROMPTHEUS_PROVIDER_ID) {
        await swapToConversationDialog(HOST_LABEL);
      } else {
        await swapAiWebview(id, HOST_LABEL);
      }
    } catch (e) {
      console.error("shell toolbar swap failed", e);
    }
  }

  async function toggleProviderDropdown(e: MouseEvent) {
    e.stopPropagation();
    if (providerDropdownOpen) {
      providerDropdownOpen = false;
      try {
        await hideProviderMenu();
      } catch (err) {
        console.error("hide_provider_menu failed", err);
      }
      return;
    }

    const btn = triggerEl;
    if (!btn) return;
    const rect = btn.getBoundingClientRect();
    const hostWin = getCurrentWindow();
    try {
      const pos = await hostWin.outerPosition();
      const scale = await hostWin.scaleFactor();
      const anchorX = pos.x / scale + rect.left;
      const anchorY = pos.y / scale + rect.bottom + 4;
      const width = Math.max(rect.width, 160);
      const height = providers.length * 28 + 8;
      providerDropdownOpen = true;
      await showProviderMenu(HOST_LABEL, anchorX, anchorY, width, height, providers, activeId);
    } catch (err) {
      providerDropdownOpen = false;
      console.error("show_provider_menu failed", err);
    }
  }

  async function handleOpenInNewWindow() {
    try {
      const providerId =
        activeId === PROMPTHEUS_PROVIDER_ID ? undefined : activeId;
      await openConversationDialogNewWindow(HOST_LABEL, providerId);
    } catch (e) {
      console.error("open in new window failed", e);
    }
  }

  async function handleMinimize() {
    try {
      await getCurrentWindow().minimize();
    } catch (e) {
      console.error("minimize failed", e);
    }
  }

  async function handleToggleMaximize() {
    try {
      await getCurrentWindow().toggleMaximize();
      isMaximized = await getCurrentWindow().isMaximized();
    } catch (e) {
      console.error("toggle maximize failed", e);
    }
  }

  async function handleClose() {
    try {
      await getCurrentWindow().close();
    } catch (e) {
      console.error("close failed", e);
    }
  }

  async function handleGlobalKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "p") {
      e.preventDefault();
      e.stopPropagation();
      try {
        await openPalette(HOST_LABEL);
      } catch (err) {
        console.error("open_palette failed", err);
      }
    }
  }

  onMount(async () => {
    window.addEventListener("keydown", handleGlobalKeydown, true);

    await refreshWebviewProviders();
    unlistenSettingsChanged = await onSettingsChanged(refreshWebviewProviders);
    await refreshActive();

    try {
      isMaximized = await getCurrentWindow().isMaximized();
    } catch {}

    unlistenActive = await listen<{ provider_id: string | null }>(
      "shell:active-changed",
      (ev) => {
        activeId = ev.payload.provider_id ?? PROMPTHEUS_PROVIDER_ID;
      },
      { target: SELF_TARGET },
    );

    unlistenSelect = await listen<{ provider_id: string }>(
      "provider-menu:select",
      (ev) => {
        providerDropdownOpen = false;
        void selectProvider(ev.payload.provider_id);
      },
      { target: SELF_TARGET },
    );

    unlistenClosed = await listen(
      "provider-menu:closed",
      () => {
        providerDropdownOpen = false;
      },
      { target: SELF_TARGET },
    );
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleGlobalKeydown, true);
    unlistenActive?.();
    unlistenSelect?.();
    unlistenClosed?.();
    unlistenSettingsChanged?.();
  });
</script>

<div class="titlebar" class:mac={isMac} data-tauri-drag-region>
  <span class="hint" title="Open command palette">{shortcutHint}</span>

  <div class="switcher">
    <button
      bind:this={triggerEl}
      type="button"
      class="trigger"
      aria-haspopup="listbox"
      aria-expanded={providerDropdownOpen}
      onmousedown={toggleProviderDropdown}
    >
      {#if activeIconSvg}
        <span class="trigger-icon" aria-hidden="true">{@html activeIconSvg}</span>
      {/if}
      <span class="trigger-label">{activeProvider?.name ?? "Promptheus"}</span>
      <ChevronDown size={14} />
    </button>
  </div>

  <button
    type="button"
    class="icon-btn"
    title="Open in new window"
    onclick={handleOpenInNewWindow}
  >
    <SquareArrowOutUpRight size={14} />
  </button>

  <div class="drag-fill" data-tauri-drag-region></div>

  {#if !isMac}
    <div class="actions">
      <button type="button" class="win-btn" title="Minimize" onclick={handleMinimize}>
        <Minus size={14} />
      </button>
      <button type="button" class="win-btn" title={isMaximized ? "Restore" : "Maximize"} onclick={handleToggleMaximize}>
        <Square size={12} />
      </button>
      <button type="button" class="win-btn close" title="Close" onclick={handleClose}>
        <X size={14} />
      </button>
    </div>
  {/if}
</div>

<style>
  :global(html),
  :global(body) {
    background: #1e1e1e;
  }

  .titlebar {
    height: 40px;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 4px 0 8px;
    background: #1e1e1e;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 12px;
    user-select: none;
    -webkit-user-select: none;
    box-sizing: border-box;
  }

  .titlebar.mac {
    padding-left: 80px;
  }

  .switcher {
    position: relative;
    display: inline-flex;
  }

  .trigger {
    appearance: none;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.04);
    color: #fff;
    padding: 4px 6px 4px 8px;
    border-radius: 6px;
    font: inherit;
    cursor: pointer;
    line-height: 1;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 110px;
  }

  .trigger-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: #fff;
  }

  .trigger-icon :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
    transform: scale(1.25);
    transform-origin: center;
  }

  .trigger-icon :global(img) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: contain;
    transform: scale(1.25);
    transform-origin: center;
    filter: brightness(0) invert(1);
  }

  .trigger:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .trigger-label {
    flex: 1;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .trigger :global(svg) {
    color: rgba(255, 255, 255, 0.55);
    flex-shrink: 0;
  }

  .drag-fill {
    flex: 1;
    align-self: stretch;
  }

  .actions {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .hint {
    color: rgba(255, 255, 255, 0.35);
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .icon-btn {
    appearance: none;
    border: 0;
    padding: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    width: 26px;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    cursor: pointer;
  }

  .icon-btn :global(svg) {
    display: block;
  }

  .icon-btn:hover:not(:disabled) {
    color: #fff;
    background: rgba(255, 255, 255, 0.08);
  }

  .icon-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .sep {
    width: 1px;
    height: 20px;
    background: rgba(255, 255, 255, 0.08);
    margin: 0 4px;
  }

  .win-btn {
    appearance: none;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.55);
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    cursor: pointer;
  }

  .win-btn:hover {
    color: #fff;
    background: rgba(255, 255, 255, 0.08);
  }

  .win-btn.close:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
  }
</style>
