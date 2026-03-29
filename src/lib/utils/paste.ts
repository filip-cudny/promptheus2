import { invoke } from "@tauri-apps/api/core";
import type { ConversationImage } from "$lib/types/conversation";

export async function getClipboardImage(): Promise<ConversationImage | null> {
  try {
    const hasImage = await invoke<boolean>("clipboard_has_image");
    if (!hasImage) return null;
    const [base64, mediaType] = await invoke<[string, string]>(
      "get_clipboard_image",
    );
    return { data: base64, media_type: mediaType };
  } catch (e) {
    console.error("Failed to read clipboard image:", e);
    return null;
  }
}
