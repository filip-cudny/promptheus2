import type { ModelConfig, Surfaces } from "$lib/types";

export function agentModelId(surfaces: Surfaces): string | null {
  return surfaces.settings_agent.generation.model_id || surfaces.chat.generation.model_id || null;
}

export function agentHasUsableModel(surfaces: Surfaces, models: ModelConfig[]): boolean {
  const id = agentModelId(surfaces);
  return id !== null && models.some((m) => m.id === id && m.type === "text");
}

export type AgentModelState = "no_text_model" | "no_usable_model" | "stale_agent_model" | "ok";

export type SurfaceModelWrite = { surface: "chat" | "settings_agent"; modelId: string | null };

export function agentModelState(surfaces: Surfaces, models: ModelConfig[]): AgentModelState {
  if (!models.some((m) => m.type === "text")) return "no_text_model";
  if (agentHasUsableModel(surfaces, models)) return "ok";
  const agentId = surfaces.settings_agent.generation.model_id;
  return agentId && !models.some((m) => m.id === agentId) ? "stale_agent_model" : "no_usable_model";
}

export function useModelWrites(
  surfaces: Surfaces,
  models: ModelConfig[],
  pickedId: string,
): SurfaceModelWrite[] {
  const agentId = surfaces.settings_agent.generation.model_id;
  const agentIdUnusable = !!agentId && !models.some((m) => m.id === agentId && m.type === "text");
  const writes: SurfaceModelWrite[] = [{ surface: "chat", modelId: pickedId }];
  return agentIdUnusable ? [{ surface: "settings_agent", modelId: null }, ...writes] : writes;
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
