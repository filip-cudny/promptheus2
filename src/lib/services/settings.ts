import { invoke } from "@tauri-apps/api/core";
import type {
  Settings,
  ModelConfig,
  NotificationSettings,
  SurfaceKind,
  SpeechToTextConfig,
  CatalogEntry,
} from "$lib/types";
import { invalidateCapabilities } from "$lib/stores/capabilities.svelte";
import { getSettingsStore } from "$lib/stores/settings.svelte";

const SETTING_SUFFIX = /^([\s\S]*) \(setting: (.+)\)$/;

export function parseSettingError(e: unknown): {
  key: string | null;
  message: string;
} {
  const text = e instanceof Error ? e.message : String(e);
  const match = SETTING_SUFFIX.exec(text);
  return match
    ? { key: match[2], message: match[1] }
    : { key: null, message: text };
}

async function invokeWrite(
  command: string,
  args: Record<string, unknown>,
): Promise<void> {
  const store = getSettingsStore();
  try {
    await invoke(command, args);
  } catch (e) {
    store.reportWriteError(e);
    throw e;
  }
  store.clearFieldErrors();
}

export async function getSettings(): Promise<Settings> {
  return invoke("get_settings");
}

export async function getSettingsCatalog(): Promise<CatalogEntry[]> {
  return invoke("get_settings_catalog");
}

export async function updateSetting(
  key: string,
  value: unknown,
): Promise<void> {
  return invokeWrite("update_setting", { key, value });
}

export async function setShortcut(
  action: string,
  shortcut: string | null,
): Promise<void> {
  return invokeWrite("set_shortcut", { action, shortcut });
}

export async function updateSurfaceModel(
  surface: SurfaceKind,
  modelId: string | null,
): Promise<void> {
  return invokeWrite("update_surface_model", { surface, modelId });
}

export async function updateSurfaceParameter(
  surface: SurfaceKind,
  key: string,
  value: unknown,
): Promise<void> {
  return invokeWrite("update_surface_parameter", { surface, key, value });
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
  return invokeWrite("update_surface_enabled_tools", { surface, tools });
}

export async function updateSpeechToTextConfig(
  config: SpeechToTextConfig,
): Promise<void> {
  return invokeWrite("update_speech_to_text_config", { config });
}

export async function setSpeechToTextModel(
  modelId: string | null,
): Promise<void> {
  return updateSurfaceModel("speech_to_text", modelId);
}

export async function addModel(config: ModelConfig): Promise<void> {
  return invokeWrite("add_model", { config });
}

export async function updateModel(
  modelId: string,
  config: ModelConfig,
): Promise<void> {
  await invokeWrite("update_model", { modelId, config });
  invalidateCapabilities(modelId);
}

export async function deleteModel(modelId: string): Promise<void> {
  await invokeWrite("delete_model", { modelId });
  invalidateCapabilities(modelId);
}

export async function updateNotifications(
  config: NotificationSettings,
): Promise<void> {
  return invokeWrite("update_notifications", { config });
}

export async function updateHistoryRetention(days: number): Promise<void> {
  return invokeWrite("update_history_retention", { days });
}
