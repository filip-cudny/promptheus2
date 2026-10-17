import { invoke } from "@tauri-apps/api/core";

export interface WebviewProvider {
  id: string;
  name: string;
  url: string;
}

export async function getWebviewProviders(): Promise<WebviewProvider[]> {
  return await invoke<WebviewProvider[]>("get_webview_providers");
}
