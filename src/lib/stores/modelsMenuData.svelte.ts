import type { MenuItem } from "$lib/types/menu";
import type { ModelCapabilities, Provider } from "$lib/types";
import { prefetchCapabilities, getCachedCapabilities } from "$lib/stores/capabilities.svelte";
import {
  updateSurfaceModel,
  updateSurfaceReasoningEffort,
  setSpeechToTextModel,
} from "$lib/services/settings";

export type MenuModel = {
  id: string;
  display_name: string;
  model: string;
  provider: Provider;
  group: string | null;
};

export interface ModelsMenuData {
  models: MenuModel[];
  default_model_id: string | null;
  default_reasoning_effort: string | null;
  stt_models: MenuModel[];
  speech_to_text_model_id: string | null;
}

function extractModelsData(item: MenuItem): ModelsMenuData | null {
  if (item.item_type !== "models") return null;
  return (item.data ?? null) as ModelsMenuData | null;
}

export function useModelsMenuData(getItems: () => MenuItem[]) {
  let defaultModelId = $state<string | null>(null);
  let reasoningEffort = $state<string | null>(null);
  let sttModelId = $state<string | null>(null);

  const modelsData = $derived.by<ModelsMenuData | null>(() => {
    const modelsItem = getItems().find((i) => i.item_type === "models");
    return modelsItem ? extractModelsData(modelsItem) : null;
  });

  const modelNames = $derived.by<Map<string, string>>(() => {
    const map = new Map<string, string>();
    if (modelsData) {
      for (const m of modelsData.models) map.set(m.id, m.display_name);
      for (const m of modelsData.stt_models) map.set(m.id, m.display_name);
    }
    return map;
  });

  const quickActionModel = $derived.by<MenuModel | null>(() => {
    if (!modelsData || !defaultModelId) return null;
    return modelsData.models.find((m) => m.id === defaultModelId) ?? null;
  });

  $effect(() => {
    const data = modelsData;
    if (data) {
      defaultModelId = data.default_model_id;
      reasoningEffort = data.default_reasoning_effort;
      sttModelId = data.speech_to_text_model_id;
    }
  });

  $effect(() => {
    if (quickActionModel) {
      prefetchCapabilities({
        id: quickActionModel.id,
        type: "text",
        model: quickActionModel.model,
        display_name: quickActionModel.display_name,
        provider: quickActionModel.provider,
        group: quickActionModel.group,
        api_key: null,
        base_url: null,
        parameters: null,
        context_window_size: null,
        api_mode: null,
        store: true,
      });
    }
  });

  const quickActionCapabilities = $derived<ModelCapabilities | null>(
    getCachedCapabilities(
      quickActionModel
        ? {
            id: quickActionModel.id,
            type: "text",
            model: quickActionModel.model,
            display_name: quickActionModel.display_name,
            provider: quickActionModel.provider,
            group: quickActionModel.group,
            api_key: null,
            base_url: null,
            parameters: null,
            context_window_size: null,
            api_mode: null,
            store: true,
          }
        : null,
    ),
  );

  return {
    get modelsData() { return modelsData; },
    get modelNames() { return modelNames; },
    get quickActionModel() { return quickActionModel; },
    get quickActionCapabilities() { return quickActionCapabilities; },
    get defaultModelId() { return defaultModelId; },
    get reasoningEffort() { return reasoningEffort; },
    get sttModelId() { return sttModelId; },
    async setDefaultModel(id: string) {
      defaultModelId = id;
      await updateSurfaceModel("quick_actions", id);
    },
    async setReasoningEffort(effort: string | null) {
      reasoningEffort = effort;
      await updateSurfaceReasoningEffort("quick_actions", effort);
    },
    async setSttModel(id: string) {
      sttModelId = id;
      await setSpeechToTextModel(id);
    },
  };
}

export type ModelsMenuDataStore = ReturnType<typeof useModelsMenuData>;
