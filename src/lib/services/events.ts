import type { UnlistenFn } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";

export interface SettingsChangedEvent {
  version: number;
}

export interface HistoryChangedEvent {
  added_id: string | null;
  removed_id: string | null;
  version: number;
}

export interface ContextChangedEvent {
  version: number;
}

export function onSettingsChanged(
  callback: (payload: SettingsChangedEvent) => void,
): Promise<UnlistenFn> {
  return listen<SettingsChangedEvent>("settings-changed", (event) =>
    callback(event.payload),
  );
}

export function onHistoryChanged(
  callback: (payload: HistoryChangedEvent) => void,
): Promise<UnlistenFn> {
  return listen<HistoryChangedEvent>("history-changed", (event) =>
    callback(event.payload),
  );
}

export function onContextChanged(
  callback: (payload: ContextChangedEvent) => void,
): Promise<UnlistenFn> {
  return listen<ContextChangedEvent>("context-changed", (event) =>
    callback(event.payload),
  );
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
