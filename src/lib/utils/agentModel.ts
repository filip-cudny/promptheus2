import type { ModelConfig, Surfaces } from "$lib/types";

export function agentModelId(surfaces: Surfaces): string | null {
  return surfaces.settings_agent.generation.model_id || surfaces.chat.generation.model_id || null;
}

export function agentModelName(surfaces: Surfaces, models: ModelConfig[]): string {
  const id = agentModelId(surfaces);
  const model = id ? models.find((m) => m.id === id) : undefined;
  return model ? model.display_name || model.model : "No model";
}
