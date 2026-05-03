<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { takePendingProvider } from "$lib/services/aiWebview";
  import { openConversationDialogNewWindow } from "$lib/services/conversationDialog";
  import { PROMPTHEUS_PROVIDER_ID, openPalette } from "$lib/services/shellToolbar";
  import { SHORTCUTS, matches } from "$lib/shortcuts";
  import Titlebar from "$lib/components/features/shell-toolbar/Titlebar.svelte";
  import { useShellToolbar } from "$lib/components/features/shell-toolbar/drivers/useShellToolbar.svelte";

  const HOST_LABEL = getCurrentWindow().label;
  const SELF_TARGET = getCurrentWebview().label;
  const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);
  const shortcutHint = isMac ? "⌘P" : "Ctrl P";

  const toolbar = useShellToolbar({ hostLabel: HOST_LABEL, selfTarget: SELF_TARGET });

  let isMaximized = $state(false);
  let triggerEl = $state<HTMLButtonElement | null>(null);

  async function handleToggleProviderDropdown(e: MouseEvent) {
    e.stopPropagation();
    if (!triggerEl) return;
    await toolbar.toggleProviderDropdown(triggerEl);
  }

  async function handleOpenInNewWindow() {
    try {
      const providerId = toolbar.activeId === PROMPTHEUS_PROVIDER_ID ? undefined : toolbar.activeId;
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
    if (matches(e, SHORTCUTS.openPalette)) {
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
    window.addEventListener("keydown", handleGlobalKeydown);
    await toolbar.init();
    try {
      isMaximized = await getCurrentWindow().isMaximized();
    } catch {}
    try {
      const pending = await takePendingProvider(HOST_LABEL);
      if (pending) await toolbar.selectProvider(pending);
    } catch (e) {
      console.error("take_pending_provider failed", e);
    }
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleGlobalKeydown);
    toolbar.destroy();
  });
</script>

<Titlebar
  {isMac}
  {shortcutHint}
  activeProvider={toolbar.activeProvider}
  providerDropdownOpen={toolbar.providerDropdownOpen}
  bind:triggerEl
  {isMaximized}
  onToggleProviderDropdown={handleToggleProviderDropdown}
  onOpenInNewWindow={handleOpenInNewWindow}
  onMinimize={handleMinimize}
  onToggleMaximize={handleToggleMaximize}
  onClose={handleClose}
/>

<style>
  :global(html),
  :global(body) {
    background: var(--surface-base);
  }
</style>
