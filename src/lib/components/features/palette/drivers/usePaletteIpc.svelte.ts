import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { tick } from "svelte";
import {
  PROMPTHEUS_PROVIDER_ID,
  closePalette,
  reloadActiveInHost,
} from "$lib/services/shellToolbar";

type ProviderPayload = { id: string; name: string; url?: string | null };

type ShowPayload = {
  host_label: string;
  active_id: string;
  providers: ProviderPayload[];
};

export function usePaletteIpc(opts: {
  onShown?: () => void;
}) {
  let hostLabel = $state("");
  let activeId = $state(PROMPTHEUS_PROVIDER_ID);
  let webviewProviders = $state<ProviderPayload[]>([]);
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

  async function init() {
    const u1 = await listen<ShowPayload>("palette:show", async (ev) => {
      hostLabel = ev.payload.host_label;
      activeId = ev.payload.active_id;
      webviewProviders = ev.payload.providers;
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
    get visible() {
      return visible;
    },
    init,
    destroy,
    dismiss,
    reloadActive,
  };
}

export type PaletteIpc = ReturnType<typeof usePaletteIpc>;
