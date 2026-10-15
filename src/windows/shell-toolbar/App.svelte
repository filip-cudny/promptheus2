<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { openPalette } from "$lib/services/shellToolbar";
  import { SHORTCUTS, matches } from "$lib/shortcuts";
  import { attachEdgeResizeCursor } from "$lib/utils/edgeResizeCursor";
  import Titlebar from "$lib/components/features/shell-toolbar/Titlebar.svelte";
  import { useShellToolbar } from "$lib/components/features/shell-toolbar/drivers/useShellToolbar.svelte";

  const HOST_LABEL = getCurrentWindow().label;
  const SELF_TARGET = getCurrentWebview().label;
  const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);

  const toolbar = useShellToolbar({ hostLabel: HOST_LABEL, selfTarget: SELF_TARGET });

  let isMaximized = $state(false);

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

  let detachEdgeResizeCursor: (() => void) | null = null;

  onMount(async () => {
    window.addEventListener("keydown", handleGlobalKeydown);
    detachEdgeResizeCursor = attachEdgeResizeCursor("toolbar");
    await toolbar.init();
    try {
      isMaximized = await getCurrentWindow().isMaximized();
    } catch {}
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleGlobalKeydown);
    if (detachEdgeResizeCursor) {
      detachEdgeResizeCursor();
      detachEdgeResizeCursor = null;
    }
    toolbar.destroy();
  });
</script>

<Titlebar
  {isMac}
  tabs={toolbar.tabs}
  activeId={toolbar.activeId}
  providers={toolbar.webviewProviders}
  {isMaximized}
  onActivate={toolbar.activate}
  onClose={toolbar.close}
  onOpenPlus={toolbar.openPlusMenu}
  onOpenOverflow={toolbar.openOverflowMenu}
  onMinimize={handleMinimize}
  onToggleMaximize={handleToggleMaximize}
  onWindowClose={handleClose}
/>

<style>
  :global(html),
  :global(body) {
    background: var(--surface-base);
  }
</style>
