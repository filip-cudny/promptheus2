import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export async function openImagePreview(data: string, mediaType: string): Promise<void> {
  await invoke("open_image_preview", { data, mediaType });
}

export async function openTextPreview(text: string, index: number): Promise<void> {
  const sourceWindow = getCurrentWebviewWindow().label;
  try {
    await invoke("open_text_preview", { text, index, sourceWindow });
  } catch (e) {
    console.error("open_text_preview failed:", e);
  }
}
