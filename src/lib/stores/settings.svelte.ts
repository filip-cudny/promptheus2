import { listen } from "@tauri-apps/api/event";
import { getSettings } from "$lib/services/settings";
import type { Settings, ModelConfig, SurfaceKind } from "$lib/types";

let settings = $state.raw<Settings | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);
let unlisten: (() => void) | null = null;
let refreshInFlight: Promise<void> | null = null;

const models = $derived<ModelConfig[]>(settings?.models ?? []);

const surfaceModelIds = $derived.by<Record<SurfaceKind, string | null>>(() => {
  const s = settings;
  if (!s) {
    return {
      chat: null,
      quick_actions: null,
      title_generation: null,
      speech_to_text: null,
    };
  }
  return {
    chat: s.surfaces.chat.generation.model_id,
    quick_actions: s.surfaces.quick_actions.generation.model_id,
    title_generation: s.surfaces.title_generation.generation.model_id,
    speech_to_text: s.surfaces.speech_to_text.model_id,
  };
});

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
    unlisten = await listen("settings-changed", () => {
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

function getModel(id: string | null): ModelConfig | null {
  if (!id) return null;
  return models.find((m) => m.id === id) ?? null;
}

function isModelReferencedBySurface(modelId: string): SurfaceKind | null {
  const ids = surfaceModelIds;
  const entries: [SurfaceKind, string | null][] = [
    ["chat", ids.chat],
    ["quick_actions", ids.quick_actions],
    ["title_generation", ids.title_generation],
    ["speech_to_text", ids.speech_to_text],
  ];
  const hit = entries.find(([, id]) => id === modelId);
  return hit ? hit[0] : null;
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
    get surfaceModelIds() {
      return surfaceModelIds;
    },
    init,
    refresh,
    destroy,
    getModel,
    isModelReferencedBySurface,
  };
}
