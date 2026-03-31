import { invoke } from "@tauri-apps/api/core";

export async function openHistoryDialog(): Promise<void> {
  await invoke("open_history_dialog");
}
