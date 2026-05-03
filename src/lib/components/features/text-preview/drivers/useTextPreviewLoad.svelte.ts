import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";

type PendingText = {
  text: string;
  index: number;
  source_window: string;
};

export function useTextPreviewLoad() {
  let index = $state(0);
  let sourceWindow = $state("");
  let originalText = $state("");

  async function load(): Promise<string | null> {
    const payload = await invoke<PendingText | null>("get_pending_text");
    if (!payload) return null;
    index = payload.index;
    sourceWindow = payload.source_window;
    originalText = payload.text;
    return payload.text;
  }

  function commitAndHide(label: string, text: string) {
    emitTo(sourceWindow, "text-attachment-updated", { text, index });
    invoke("save_text_preview_geometry");
    invoke("hide_dialog_window", { label });
  }

  return {
    get index() {
      return index;
    },
    get sourceWindow() {
      return sourceWindow;
    },
    get originalText() {
      return originalText;
    },
    load,
    commitAndHide,
  };
}

export type TextPreviewLoad = ReturnType<typeof useTextPreviewLoad>;
