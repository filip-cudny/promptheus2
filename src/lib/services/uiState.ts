import { invoke } from "@tauri-apps/api/core";

export async function getUiState<T>(key: string): Promise<T | null> {
  return invoke("get_ui_state", { key });
}

export async function setUiState(
  key: string,
  value: unknown,
): Promise<void> {
  return invoke("set_ui_state", { key, value });
}
