import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getWebviewProviders, type WebviewProvider } from "$lib/services/aiWebview";
import { onSettingsChanged } from "$lib/services/events";
import {
  PROMPTHEUS_PROVIDER_ID,
  hideProviderMenu,
  showProviderMenu,
  type ProviderMenuItem,
} from "$lib/services/shellToolbar";
import {
  activateTab,
  closeTab,
  getTabs,
  newTab,
  type TabDto,
  type TabsSnapshot,
} from "$lib/services/tabs";

const MENU_ITEM_HEIGHT = 28;
const MENU_MIN_WIDTH = 220;

export function useShellToolbar(opts: {
  hostLabel: string;
  selfTarget: string;
}) {
  let snapshot = $state<TabsSnapshot>({ tabs: [], active_id: null });
  let webviewProviders = $state<WebviewProvider[]>([]);
  let menuOpen = $state(false);
  let menuAnchor: HTMLElement | null = null;
  let unlistens: UnlistenFn[] = [];

  async function refreshWebviewProviders() {
    try {
      webviewProviders = await getWebviewProviders();
    } catch (e) {
      console.error("getWebviewProviders failed", e);
    }
  }

  async function refreshTabs() {
    try {
      snapshot = await getTabs();
    } catch (e) {
      console.error("get_tabs failed", e);
    }
  }

  async function run(label: string, action: () => Promise<void>) {
    try {
      await action();
    } catch (e) {
      console.error(`${label} failed`, e);
    }
  }

  const activate = (id: string) => run("activate_tab", () => activateTab(id));
  const close = (id: string) => run("close_tab", () => closeTab(id));
  const openNewTab = (providerId: string) => run("new_tab", () => newTab(providerId));

  async function toggleMenu(anchor: HTMLElement, items: ProviderMenuItem[]) {
    if (menuOpen && menuAnchor === anchor) {
      menuOpen = false;
      await run("hide_provider_menu", hideProviderMenu);
      return;
    }
    menuAnchor = anchor;
    const rect = anchor.getBoundingClientRect();
    const hostWin = getCurrentWindow();
    try {
      const pos = await hostWin.outerPosition();
      const scale = await hostWin.scaleFactor();
      const offsetX = rect.left;
      const offsetY = rect.bottom + 4;
      menuOpen = true;
      await showProviderMenu(
        opts.hostLabel,
        pos.x / scale + offsetX,
        pos.y / scale + offsetY,
        offsetX,
        offsetY,
        Math.max(rect.width, MENU_MIN_WIDTH),
        items.length * MENU_ITEM_HEIGHT + 8,
        items,
      );
    } catch (err) {
      menuOpen = false;
      console.error("show_provider_menu failed", err);
    }
  }

  function openPlusMenu(anchor: HTMLElement) {
    return toggleMenu(anchor, [
      { id: PROMPTHEUS_PROVIDER_ID, kind: "provider", icon: PROMPTHEUS_PROVIDER_ID, title: "Promptheus", active: false },
      ...webviewProviders.map((p) => ({
        id: p.id,
        kind: "provider" as const,
        icon: p.id,
        title: p.name,
        active: false,
      })),
    ]);
  }

  function openOverflowMenu(anchor: HTMLElement, hidden: TabDto[]) {
    return toggleMenu(
      anchor,
      hidden.map((t) => ({
        id: t.id,
        kind: "tab" as const,
        icon: t.provider_id ?? PROMPTHEUS_PROVIDER_ID,
        title: t.title,
        active: t.id === snapshot.active_id,
      })),
    );
  }

  async function init() {
    const target = { target: opts.selfTarget };
    const onTabs = await listen<TabsSnapshot>(
      "shell:tabs-changed",
      (ev) => {
        snapshot = ev.payload;
      },
      target,
    );
    const onSelect = await listen<{ item_id: string }>(
      "provider-menu:select",
      (ev) => {
        menuOpen = false;
        const id = ev.payload.item_id;
        if (snapshot.tabs.some((t) => t.id === id)) void activate(id);
        else void openNewTab(id);
      },
      target,
    );
    const onClosed = await listen(
      "provider-menu:closed",
      () => {
        menuOpen = false;
      },
      target,
    );
    const onSettings = await onSettingsChanged(() => {
      void refreshWebviewProviders();
    });
    unlistens = [onTabs, onSelect, onClosed, onSettings];
    await refreshWebviewProviders();
    await refreshTabs();
  }

  function destroy() {
    for (const fn of unlistens) fn();
    unlistens = [];
  }

  return {
    get tabs() {
      return snapshot.tabs;
    },
    get activeId() {
      return snapshot.active_id;
    },
    get webviewProviders() {
      return webviewProviders;
    },
    init,
    destroy,
    activate,
    close,
    newTab: openNewTab,
    openPlusMenu,
    openOverflowMenu,
  };
}

export type ShellToolbar = ReturnType<typeof useShellToolbar>;
