import { invoke } from "@tauri-apps/api/core";

import { SHORTCUTS, matches, tabPosition } from "$lib/shortcuts";

export interface TabDto {
  id: string;
  kind: "native" | "provider";
  provider_id: string | null;
  native_tab_id: string | null;
  title: string;
  live: boolean;
  streaming: boolean;
  recency: number;
}

export interface TabsSnapshot {
  tabs: TabDto[];
  active_id: string | null;
}

export interface NativeTabReport {
  tab_id: string;
  title: string;
  streaming: boolean;
  listed: boolean;
}

export async function getTabs(): Promise<TabsSnapshot> {
  return await invoke<TabsSnapshot>("get_tabs");
}

export async function activateTab(tabId: string): Promise<void> {
  await invoke("activate_tab", { tabId });
}

export async function closeTab(tabId: string): Promise<void> {
  await invoke("close_tab", { tabId });
}

export async function newTab(providerId: string): Promise<void> {
  await invoke("new_tab", { providerId });
}

export async function reportNativeTabs(
  tabs: NativeTabReport[],
  activeTabId: string | null,
): Promise<void> {
  await invoke("report_native_tabs", { tabs, activeTabId });
}

export async function activateAdjacentTab(step: number): Promise<void> {
  await invoke("activate_adjacent_tab", { step });
}

export async function activateTabAt(position: number): Promise<void> {
  await invoke("activate_tab_at", { position });
}

export async function closeActiveTab(): Promise<void> {
  await invoke("close_active_tab");
}

export async function newTabLikeActive(): Promise<void> {
  await invoke("new_tab_like_active");
}

export function tabShortcutAction(e: KeyboardEvent): (() => Promise<void>) | null {
  if (matches(e, SHORTCUTS.nextTab)) return () => activateAdjacentTab(1);
  if (matches(e, SHORTCUTS.prevTab)) return () => activateAdjacentTab(-1);
  if (matches(e, SHORTCUTS.newTab)) return newTabLikeActive;
  if (matches(e, SHORTCUTS.closeTab)) return closeActiveTab;
  const position = tabPosition(e);
  return position === null ? null : () => activateTabAt(position);
}
