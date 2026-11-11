<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { createHostShortcutHandler } from "$lib/shortcuts";
  import { isMac } from "$lib/utils/platform";
  import { attachEdgeResizeCursor } from "$lib/utils/edgeResizeCursor";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getChatCorners } from "$lib/services/shellToolbar";
  import { initLogo } from "$lib/stores/logo.svelte";
  import Titlebar from "$lib/components/features/shell-toolbar/Titlebar.svelte";
  import { useShellToolbar } from "$lib/components/features/shell-toolbar/drivers/useShellToolbar.svelte";

  const HOST_LABEL = getCurrentWindow().label;
  const SELF_TARGET = getCurrentWebview().label;

  const toolbar = useShellToolbar({ hostLabel: HOST_LABEL, selfTarget: SELF_TARGET });

  let isMaximized = $state(false);
  let rounded = $state(false);
  let unlistenCorners: UnlistenFn | null = null;

  $effect(() => {
    document.documentElement.classList.toggle("rounded", rounded);
  });

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

  const handleGlobalKeydown = createHostShortcutHandler(HOST_LABEL);

  let detachEdgeResizeCursor: (() => void) | null = null;

  onMount(async () => {
    initLogo();
    window.addEventListener("keydown", handleGlobalKeydown);
    detachEdgeResizeCursor = attachEdgeResizeCursor("toolbar");
    await toolbar.init();
    unlistenCorners = await listen<{ rounded: boolean }>(
      "shell:corners",
      (ev) => {
        rounded = ev.payload.rounded;
      },
      { target: SELF_TARGET },
    );
    try {
      rounded = await getChatCorners();
    } catch (e) {
      console.error("get_chat_corners failed", e);
    }
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
    unlistenCorners?.();
    unlistenCorners = null;
    toolbar.destroy();
  });
</script>

<Titlebar
  {isMac}
  tabs={toolbar.tabs}
  activeId={toolbar.activeId}
  providers={toolbar.webviewProviders}
  {isMaximized}
  {rounded}
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

  :global(html.rounded),
  :global(html.rounded body) {
    background: transparent;
  }
</style>
