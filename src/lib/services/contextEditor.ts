import { invoke } from "@tauri-apps/api/core";

export async function openContextEditor(): Promise<void> {
  await invoke("open_context_editor");
}
