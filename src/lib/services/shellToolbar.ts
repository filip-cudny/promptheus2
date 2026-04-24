import { invoke } from "@tauri-apps/api/core";

export interface ShellProvider {
  id: string;
  name: string;
}

export const PROMPTHEUS_PROVIDER_ID = "promptheus";
export const CONVERSATION_DIALOG_LABEL = "conversation-dialog";

export async function getActiveProvider(hostLabel: string): Promise<string | null> {
  return await invoke<string | null>("get_active_provider", { hostLabel });
}

export async function openPalette(hostLabel: string): Promise<void> {
  await invoke("open_palette", { hostLabel });
}

export async function newChatInHost(hostLabel: string): Promise<void> {
  await invoke("new_chat_in_host", { hostLabel });
}

export async function closePalette(
  hostLabel: string,
  selectedProviderId: string | null,
): Promise<void> {
  await invoke("close_palette", { hostLabel, selectedProviderId });
}

export async function showProviderMenu(
  anchorX: number,
  anchorY: number,
  width: number,
  height: number,
  providers: { id: string; name: string }[],
  activeId: string,
): Promise<void> {
  await invoke("show_provider_menu", {
    anchorX,
    anchorY,
    width,
    height,
    providers,
    activeId,
  });
}

export async function hideProviderMenu(): Promise<void> {
  await invoke("hide_provider_menu");
}
