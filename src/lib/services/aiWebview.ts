import { invoke } from "@tauri-apps/api/core";

export interface AiProvider {
  id: string;
  name: string;
  url: string;
}

export async function getAiProviders(): Promise<AiProvider[]> {
  return await invoke<AiProvider[]>("get_ai_providers");
}

export async function openAiWebview(providerId: string, url?: string): Promise<void> {
  await invoke("open_ai_webview", { providerId, url: url ?? null });
}

export async function openAiWebviewNewWindow(providerId: string, url?: string): Promise<void> {
  await invoke("open_ai_webview_new_window", { providerId, url: url ?? null });
}

export async function swapAiWebview(providerId: string, fromLabel: string): Promise<void> {
  await invoke("swap_ai_webview", { providerId, fromLabel });
}

export async function swapToConversationDialog(fromLabel: string): Promise<void> {
  await invoke("swap_to_conversation_dialog", { fromLabel });
}

export async function navigateAiWebview(providerId: string, url: string): Promise<void> {
  await invoke("navigate_ai_webview", { providerId, url });
}

export async function closeAiWebview(providerId: string): Promise<void> {
  await invoke("close_ai_webview", { providerId });
}
