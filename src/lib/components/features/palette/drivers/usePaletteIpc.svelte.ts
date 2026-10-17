import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { tick } from "svelte";
import {
  PROMPTHEUS_PROVIDER_ID,
  closePalette,
  reloadActiveInHost,
} from "$lib/services/shellToolbar";
import { activateTab, type TabDto, type TabsSnapshot } from "$lib/services/tabs";

type ProviderPayload = { id: string; name: string; url?: string | null };

export type TabEntry = TabDto & { is_current: boolean };

type ShowPayload = {
  host_label: string;
  active_id: string;
  providers: ProviderPayload[];
  tabs: TabsSnapshot;
  current_title: string;
};

export function usePaletteIpc(opts: {
  onShown?: () => void;
}) {
  let hostLabel = $state("");
  let activeId = $state(PROMPTHEUS_PROVIDER_ID);
  let webviewProviders = $state<ProviderPayload[]>([]);
  let openTabs = $state<TabEntry[]>([]);
  let currentTitle = $state("");
  let visible = $state(false);

  let unlistens: UnlistenFn[] = [];

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
      await reloadActiveInHost();
    } catch (e) {
      console.error("reload_active_in_host failed", e);
    }
    await dismiss(null);
  }

  async function focusWindow(tabId: string) {
    if (!hostLabel) return;
    visible = false;
    try {
      await closePalette(hostLabel, null);
    } catch (e) {
      console.error("close_palette failed", e);
    }
    try {
      await activateTab(tabId);
    } catch (e) {
      console.error("activate_tab failed", e);
    }
  }

  async function init() {
    const u1 = await listen<ShowPayload>("palette:show", async (ev) => {
      hostLabel = ev.payload.host_label;
      activeId = ev.payload.active_id;
      webviewProviders = ev.payload.providers;
      openTabs = (ev.payload.tabs?.tabs ?? []).map((tab) => ({
        ...tab,
        is_current: tab.id === ev.payload.tabs.active_id,
      }));
      currentTitle = ev.payload.current_title ?? "";
      visible = false;
      await tick();
      visible = true;
      await tick();
      opts.onShown?.();
    });

    const u2 = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (!focused) dismiss(null);
    });

    const u3 = await listen("menu:reload-active", () => {
      reloadActive();
    });

    unlistens = [u1, u2, u3];
  }

  function destroy() {
    for (const fn of unlistens) fn();
    unlistens = [];
  }

  return {
    get hostLabel() {
      return hostLabel;
    },
    get activeId() {
      return activeId;
    },
    get webviewProviders() {
      return webviewProviders;
    },
    get openWindows() {
      return openTabs;
    },
    get currentTitle() {
      return currentTitle;
    },
    get visible() {
      return visible;
    },
    init,
    destroy,
    dismiss,
    reloadActive,
    focusWindow,
  };
}

export type PaletteIpc = ReturnType<typeof usePaletteIpc>;
