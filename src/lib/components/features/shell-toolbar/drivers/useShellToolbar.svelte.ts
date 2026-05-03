import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  getWebviewProviders,
  swapAiWebview,
  swapToConversationDialog,
  type WebviewProvider,
} from "$lib/services/aiWebview";
import { onSettingsChanged } from "$lib/services/events";
import {
  PROMPTHEUS_PROVIDER_ID,
  getActiveProvider,
  hideProviderMenu,
  showProviderMenu,
} from "$lib/services/shellToolbar";

type Provider = { id: string; name: string; url?: string };

export function useShellToolbar(opts: {
  hostLabel: string;
  selfTarget: string;
}) {
  let webviewProviders = $state<WebviewProvider[]>([]);
  let activeId = $state<string>(PROMPTHEUS_PROVIDER_ID);
  let providerDropdownOpen = $state(false);

  let providers = $derived<Provider[]>([
    { id: PROMPTHEUS_PROVIDER_ID, name: "Promptheus" },
    ...webviewProviders.map((p) => ({ id: p.id, name: p.name, url: p.url })),
  ]);

  let activeProvider = $derived(
    providers.find((p) => p.id === activeId) ?? providers[0],
  );

  let unlistens: UnlistenFn[] = [];

  async function refreshWebviewProviders() {
    try {
      webviewProviders = await getWebviewProviders();
    } catch (e) {
      console.error("getWebviewProviders failed", e);
    }
  }

  async function refreshActive() {
    try {
      const pid = await getActiveProvider(opts.hostLabel);
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
        await swapToConversationDialog(opts.hostLabel);
      } else {
        await swapAiWebview(id, opts.hostLabel);
      }
    } catch (e) {
      console.error("shell toolbar swap failed", e);
    }
  }

  async function toggleProviderDropdown(triggerEl: HTMLButtonElement) {
    if (providerDropdownOpen) {
      providerDropdownOpen = false;
      try {
        await hideProviderMenu();
      } catch (err) {
        console.error("hide_provider_menu failed", err);
      }
      return;
    }
    const rect = triggerEl.getBoundingClientRect();
    const hostWin = getCurrentWindow();
    try {
      const pos = await hostWin.outerPosition();
      const scale = await hostWin.scaleFactor();
      const anchorX = pos.x / scale + rect.left;
      const anchorY = pos.y / scale + rect.bottom + 4;
      const width = Math.max(rect.width, 160);
      const height = providers.length * 28 + 8;
      providerDropdownOpen = true;
      await showProviderMenu(opts.hostLabel, anchorX, anchorY, width, height, providers, activeId);
    } catch (err) {
      providerDropdownOpen = false;
      console.error("show_provider_menu failed", err);
    }
  }

  async function init() {
    const u1 = await listen<{ provider_id: string | null }>(
      "shell:active-changed",
      (ev) => {
        activeId = ev.payload.provider_id ?? PROMPTHEUS_PROVIDER_ID;
      },
      { target: opts.selfTarget },
    );
    const u2 = await listen<{ provider_id: string }>(
      "provider-menu:select",
      (ev) => {
        providerDropdownOpen = false;
        void selectProvider(ev.payload.provider_id);
      },
      { target: opts.selfTarget },
    );
    const u3 = await listen(
      "provider-menu:closed",
      () => {
        providerDropdownOpen = false;
      },
      { target: opts.selfTarget },
    );
    const u4 = await onSettingsChanged(() => {
      void refreshWebviewProviders();
    });
    unlistens = [u1, u2, u3, u4];
    await refreshWebviewProviders();
    await refreshActive();
  }

  function destroy() {
    for (const fn of unlistens) fn();
    unlistens = [];
  }

  return {
    get activeId() {
      return activeId;
    },
    get providers() {
      return providers;
    },
    get activeProvider() {
      return activeProvider;
    },
    get providerDropdownOpen() {
      return providerDropdownOpen;
    },
    init,
    destroy,
    selectProvider,
    toggleProviderDropdown,
  };
}

export type ShellToolbar = ReturnType<typeof useShellToolbar>;
