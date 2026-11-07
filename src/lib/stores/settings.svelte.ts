import { onSettingsChanged } from "$lib/services/events";
import { getSettings, parseSettingError } from "$lib/services/settings";
import { getSkillsStore } from "$lib/stores/skills.svelte";
import { modelUses, type ModelUse } from "$lib/utils/modelUses";
import type { Settings, ModelConfig } from "$lib/types";

const skills = getSkillsStore();

let settings = $state.raw<Settings | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);
let fieldErrors = $state<Record<string, string>>({});
let toastError = $state<string | null>(null);
let unlisten: (() => void) | null = null;
let refreshInFlight: Promise<void> | null = null;

const models = $derived<ModelConfig[]>(settings?.models ?? []);

const usesByModel = $derived.by<Map<string, ModelUse[]>>(() =>
  settings ? modelUses(settings.surfaces, skills.full) : new Map(),
);

async function refresh() {
  if (refreshInFlight) return refreshInFlight;
  loading = true;
  refreshInFlight = (async () => {
    try {
      settings = await getSettings();
      error = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
      refreshInFlight = null;
    }
  })();
  return refreshInFlight;
}

async function init() {
  if (!unlisten) {
    unlisten = await onSettingsChanged(() => {
      refresh();
    });
  }
  await refresh();
}

function destroy() {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
}

function fieldError(key: string): string | undefined {
  return fieldErrors[key];
}

function reportWriteError(e: unknown) {
  const { key, message } = parseSettingError(e);
  if (key) {
    fieldErrors = { ...fieldErrors, [key]: message };
  } else {
    toastError = message;
  }
}

function clearFieldErrors() {
  fieldErrors = {};
}

function dismissToast() {
  toastError = null;
}

function getModel(id: string | null): ModelConfig | null {
  if (!id) return null;
  return models.find((m) => m.id === id) ?? null;
}

function getUsesForModel(id: string): ModelUse[] {
  return usesByModel.get(id) ?? [];
}

export function getSettingsStore() {
  return {
    get settings() {
      return settings;
    },
    get models() {
      return models;
    },
    get loading() {
      return loading;
    },
    get error() {
      return error;
    },
    get usesByModel() {
      return usesByModel;
    },
    get fieldErrors() {
      return fieldErrors;
    },
    get toastError() {
      return toastError;
    },
    fieldError,
    reportWriteError,
    clearFieldErrors,
    dismissToast,
    init,
    refresh,
    destroy,
    getModel,
    getUsesForModel,
  };
}
