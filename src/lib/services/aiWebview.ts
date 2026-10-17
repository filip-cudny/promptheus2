import { invoke } from "@tauri-apps/api/core";
import type { WebviewProvider } from "$lib/types";

export type { WebviewProvider };

export async function getWebviewProviders(): Promise<WebviewProvider[]> {
  return await invoke<WebviewProvider[]>("get_webview_providers");
}
