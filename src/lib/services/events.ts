import type { UnlistenFn } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";

export function onSettingsChanged(callback: () => void): Promise<UnlistenFn> {
  return listen("settings-changed", callback);
}

export function onHistoryChanged(callback: () => void): Promise<UnlistenFn> {
  return listen("history-changed", callback);
}

export function onExecutionStarted(
  callback: (payload: { execution_id: string }) => void,
): Promise<UnlistenFn> {
  return listen<{ execution_id: string }>("execution-started", (event) =>
    callback(event.payload),
  );
}

export function onExecutionCompleted(
  callback: (payload: {
    execution_id: string;
    success: boolean;
    error: string | null;
  }) => void,
): Promise<UnlistenFn> {
  return listen<{
    execution_id: string;
    success: boolean;
    error: string | null;
  }>("execution-completed", (event) => callback(event.payload));
}
