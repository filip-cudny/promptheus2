import { invoke } from "@tauri-apps/api/core";
import type { ConversationImage } from "$lib/types/conversation";

function readFileAsBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result as string;
      const base64 = result.split(",")[1];
      resolve(base64);
    };
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

function getImageFromClipboardData(
  e: ClipboardEvent,
): DataTransferItem | null {
  const items = e.clipboardData?.items;
  if (!items) return null;
  for (const item of items) {
    if (item.type.startsWith("image/")) return item;
  }
  return null;
}

async function getImageViaArboard(): Promise<ConversationImage | null> {
  try {
    const hasImage = await invoke<boolean>("clipboard_has_image");
    if (!hasImage) return null;
    const [base64, mediaType] = await invoke<[string, string]>(
      "get_clipboard_image",
    );
    return { data: base64, media_type: mediaType };
  } catch {
    return null;
  }
}

export async function getImageFromPasteEvent(
  e: ClipboardEvent,
): Promise<ConversationImage | null> {
  const item = getImageFromClipboardData(e);
  if (item) {
    const file = item.getAsFile();
    if (file) {
      const data = await readFileAsBase64(file);
      return { data, media_type: item.type };
    }
  }

  if (hasTextInClipboardData(e)) return null;

  return getImageViaArboard();
}

function hasTextInClipboardData(e: ClipboardEvent): boolean {
  const items = e.clipboardData?.items;
  if (!items) return false;
  for (const item of items) {
    if (item.type === "text/plain") return true;
  }
  return false;
}
