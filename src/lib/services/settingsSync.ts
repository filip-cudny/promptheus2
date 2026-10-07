import { invoke } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";

export type SyncState = "off" | "synced" | "syncing" | "conflict" | "error";

export interface SyncStatus {
  state: SyncState;
  message: string | null;
  repo_path: string | null;
  branch: string | null;
  last_sync: string | null;
}

export async function getSettingsSyncStatus(): Promise<SyncStatus> {
  return invoke("get_settings_sync_status");
}

export async function syncSettingsNow(): Promise<SyncStatus> {
  return invoke("sync_settings_now");
}

export function onSettingsSyncStatusChanged(
  callback: (status: SyncStatus) => void,
): Promise<UnlistenFn> {
  return listen<SyncStatus>("settings-sync-status-changed", (event) =>
    callback(event.payload),
  );
}
