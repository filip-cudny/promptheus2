import { invoke } from "@tauri-apps/api/core";

export interface ProviderMenuItem {
  id: string;
  kind: "provider" | "tab";
  icon: string;
  title: string;
  active: boolean;
}

export const PROMPTHEUS_PROVIDER_ID = "promptheus";

export async function openPalette(hostLabel: string): Promise<void> {
  await invoke("open_palette", { hostLabel });
}

export async function reloadActiveInHost(): Promise<void> {
  await invoke("reload_active_in_host");
}

export async function closePalette(
  hostLabel: string,
  selectedProviderId: string | null,
): Promise<void> {
  await invoke("close_palette", { hostLabel, selectedProviderId });
}

export async function showProviderMenu(
  hostLabel: string,
  anchorX: number,
  anchorY: number,
  offsetX: number,
  offsetY: number,
  width: number,
  height: number,
  items: ProviderMenuItem[],
): Promise<void> {
  await invoke("show_provider_menu", {
    hostLabel,
    anchorX,
    anchorY,
    offsetX,
    offsetY,
    width,
    height,
    items,
  });
}

export async function hideProviderMenu(): Promise<void> {
  await invoke("hide_provider_menu");
}
