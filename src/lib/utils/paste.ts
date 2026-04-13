import { invoke } from "@tauri-apps/api/core";
import type { ConversationImage } from "$lib/types/conversation";

async function readFileAsBase64(file: File): Promise<string> {
  const buffer = await file.arrayBuffer();
  const bytes = new Uint8Array(buffer);
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
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
  if (hasTextInClipboardData(e)) return null;

  const item = getImageFromClipboardData(e);
  if (item) {
    const file = item.getAsFile();
    if (file) {
      const data = await readFileAsBase64(file);
      return { data, media_type: item.type };
    }
  }

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

export function extractTextAttachment(
  e: ClipboardEvent,
  threshold: number,
): string | null {
  const text = e.clipboardData?.getData("text/plain");
  if (!text || text.length < threshold) return null;
  e.preventDefault();
  return text;
}
