import { invoke } from "@tauri-apps/api/core";

export async function openSettingsWindow(section?: string): Promise<void> {
  await invoke("open_settings_window", { section: section ?? null });
}

export function saveSecret(name: string, value: string): Promise<void> {
  return invoke("save_secret", { name, value });
}

export function getSecretValue(name: string): Promise<string | null> {
  return invoke<string | null>("get_secret_value", { name });
}
