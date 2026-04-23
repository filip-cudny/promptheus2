import { invoke } from "@tauri-apps/api/core";
import type {
  Settings,
  ModelConfig,
  NotificationSettings,
  KeymapGroup,
  SurfaceKind,
  SpeechToTextConfig,
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

export async function updateSurfaceModel(
  surface: SurfaceKind,
  modelId: string | null,
): Promise<void> {
  return invoke("update_surface_model", { surface, modelId });
}

export async function updateSurfaceParameter(
  surface: SurfaceKind,
  key: string,
  value: unknown,
): Promise<void> {
  return invoke("update_surface_parameter", { surface, key, value });
}

export async function updateSurfaceReasoningEffort(
  surface: SurfaceKind,
  effort: string | null,
): Promise<void> {
  return updateSurfaceParameter(surface, "reasoning_effort", effort);
}

export async function updateSurfaceEnabledTools(
  surface: SurfaceKind,
  tools: string[],
): Promise<void> {
  return invoke("update_surface_enabled_tools", { surface, tools });
}

export async function updateSpeechToTextConfig(
  config: SpeechToTextConfig,
): Promise<void> {
  return invoke("update_speech_to_text_config", { config });
}

export async function setSpeechToTextModel(
  modelId: string | null,
): Promise<void> {
  return updateSurfaceModel("speech_to_text", modelId);
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
