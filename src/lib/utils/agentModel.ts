import type { ModelConfig, Surfaces } from "$lib/types";

export function agentModelId(surfaces: Surfaces): string | null {
  return surfaces.settings_agent.generation.model_id || surfaces.chat.generation.model_id || null;
}

export function agentReasoningEffort(surfaces: Surfaces): string | null {
  return (
    surfaces.settings_agent.generation.parameters.reasoning_effort ??
    surfaces.chat.generation.parameters.reasoning_effort ??
    null
  );
}

export function agentModelName(surfaces: Surfaces, models: ModelConfig[]): string {
  const id = agentModelId(surfaces);
  const model = id ? models.find((m) => m.id === id) : undefined;
  return model ? model.display_name || model.model : "No model";
}

export function agentModelHasBuiltinSearch(surfaces: Surfaces, models: ModelConfig[]): boolean {
  const id = agentModelId(surfaces);
  const model = id ? models.find((m) => m.id === id) : undefined;
  if (!model) return false;
  return (
    (model.provider === null || model.provider === "openai") &&
    (model.api_mode === null || model.api_mode === "responses")
  );
}
