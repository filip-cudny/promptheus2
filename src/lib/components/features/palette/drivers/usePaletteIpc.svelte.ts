import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { tick } from "svelte";
import {
  PROMPTHEUS_PROVIDER_ID,
  closePalette,
  focusWindowByLabel,
  reloadActiveInHost,
} from "$lib/services/shellToolbar";

type ProviderPayload = { id: string; name: string; url?: string | null };

export type WindowEntry = {
  host_label: string;
  kind: "promptheus" | "ai_provider";
  provider_id: string | null;
  provider_name: string | null;
  provider_url: string | null;
  title: string;
  is_current: boolean;
};

type ShowPayload = {
  host_label: string;
  active_id: string;
  providers: ProviderPayload[];
  windows: WindowEntry[];
  current_title: string;
};

export function usePaletteIpc(opts: {
  onShown?: () => void;
}) {
  let hostLabel = $state("");
  let activeId = $state(PROMPTHEUS_PROVIDER_ID);
  let webviewProviders = $state<ProviderPayload[]>([]);
  let openWindows = $state<WindowEntry[]>([]);
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
      await reloadActiveInHost(hostLabel);
    } catch (e) {
      console.error("reload_active_in_host failed", e);
    }
    await dismiss(null);
  }

  async function focusWindow(label: string) {
    if (!hostLabel) return;
    visible = false;
    try {
      await closePalette(hostLabel, null);
    } catch (e) {
      console.error("close_palette failed", e);
    }
    try {
      await focusWindowByLabel(label);
    } catch (e) {
      console.error("focus_window_by_label failed", e);
    }
  }

  async function init() {
    const u1 = await listen<ShowPayload>("palette:show", async (ev) => {
      hostLabel = ev.payload.host_label;
      activeId = ev.payload.active_id;
      webviewProviders = ev.payload.providers;
      openWindows = ev.payload.windows ?? [];
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
      return openWindows;
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
