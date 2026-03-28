import type { UnlistenFn } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";

export function onSettingsChanged(callback: () => void): Promise<UnlistenFn> {
  return listen("settings-changed", callback);
}

export function onHistoryChanged(callback: () => void): Promise<UnlistenFn> {
  return listen("history-changed", callback);
}
