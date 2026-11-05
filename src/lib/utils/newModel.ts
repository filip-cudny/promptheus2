import type { ModelConfig, ModelType, Provider } from "$lib/types";

export interface NewModelDraft {
  type: ModelType;
  display_name: string;
  provider: Provider | null;
  model: string;
  api_key: string;
  base_url: string;
}

export function newModelErrors(draft: NewModelDraft): Record<string, string> {
  const errors: Record<string, string> = {};
  if (!draft.model.trim()) errors.model = "Required";
  if (!draft.display_name.trim()) errors.display_name = "Required";
  if (!draft.api_key.trim()) errors.api_key = "Required";
  const baseUrl = draft.base_url.trim();
  if (baseUrl && !/^https?:\/\//.test(baseUrl)) {
    errors.base_url = "Must start with http:// or https://";
  }
  return errors;
}

export function buildNewModel(draft: NewModelDraft, id: string): ModelConfig {
  return {
    id,
    model: draft.model.trim(),
    display_name: draft.display_name.trim(),
    type: draft.type,
    provider: draft.provider,
    group: null,
    api_key: draft.api_key.trim(),
    base_url: draft.base_url.trim() || null,
    parameters: null,
    context_window_size: null,
    api_mode: null,
    store: true,
  };
}
