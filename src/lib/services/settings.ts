import { invoke } from "@tauri-apps/api/core";
import type {
  Settings,
  ModelConfig,
  NotificationSettings,
  SpeechToTextModel,
  KeymapGroup,
} from "$lib/types";

export async function getSettings(): Promise<Settings> {
  return invoke("get_settings");
}

export async function updateSetting(
  key: string,
  value: unknown,
): Promise<void> {
  return invoke("update_setting", { key, value });
}

export async function addModel(config: ModelConfig): Promise<void> {
  return invoke("add_model", { config });
}

export async function updateModel(
  modelId: string,
  config: ModelConfig,
): Promise<void> {
  return invoke("update_model", { modelId, config });
}

export async function deleteModel(modelId: string): Promise<void> {
  return invoke("delete_model", { modelId });
}

export async function updateNotifications(
  config: NotificationSettings,
): Promise<void> {
  return invoke("update_notifications", { config });
}

export async function updateSpeechModel(
  config: SpeechToTextModel,
): Promise<void> {
  return invoke("update_speech_model", { config });
}

export async function updateKeymaps(keymaps: KeymapGroup[]): Promise<void> {
  return invoke("update_keymaps", { keymaps });
}

export async function updateMenuSectionOrder(
  order: string[],
): Promise<void> {
  return invoke("update_menu_section_order", { order });
}

export async function reloadSettings(): Promise<void> {
  return invoke("reload_settings");
}
