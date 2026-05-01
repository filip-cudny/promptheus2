import { invoke } from "@tauri-apps/api/core";

export type ClipboardPaste =
  | { kind: "text"; text: string }
  | { kind: "image"; data: string; mediaType: string }
  | null;

export async function readClipboardForEditable(): Promise<ClipboardPaste> {
  try {
    const text = await invoke<string>("get_clipboard_text");
    if (text) return { kind: "text", text };
  } catch {}
  try {
    const [data, mediaType] = await invoke<[string, string]>("get_clipboard_image");
    if (data) return { kind: "image", data, mediaType };
  } catch {}
  return null;
}
