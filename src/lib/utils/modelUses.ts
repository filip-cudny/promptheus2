import type { ModelConfig, SurfaceKind, Surfaces } from "$lib/types";

export type ModelUse =
  | { kind: "surface"; surface: SurfaceKind; inherited: boolean }
  | { kind: "skill"; skill: string };

const INHERITING_SURFACES: SurfaceKind[] = ["quick_actions", "title_generation", "settings_agent"];

function storedModelIds(surfaces: Surfaces): Record<SurfaceKind, string | null> {
  return {
    chat: surfaces.chat.generation.model_id,
    quick_actions: surfaces.quick_actions.generation.model_id,
    title_generation: surfaces.title_generation.generation.model_id,
    settings_agent: surfaces.settings_agent.generation.model_id,
    speech_to_text: surfaces.speech_to_text.model_id,
  };
}

export function quickActionsModelId(surfaces: Surfaces): string | null {
  return surfaces.quick_actions.generation.model_id || surfaces.chat.generation.model_id || null;
}

export function modelUses(
  surfaces: Surfaces,
  skills: { name: string; model: string | null }[],
): Map<string, ModelUse[]> {
  const map = new Map<string, ModelUse[]>();
  const add = (modelId: string, use: ModelUse) => {
    const list = map.get(modelId) ?? [];
    list.push(use);
    map.set(modelId, list);
  };

  const ids = storedModelIds(surfaces);
  const surfaceOrder: SurfaceKind[] = [
    "chat",
    "quick_actions",
    "title_generation",
    "settings_agent",
    "speech_to_text",
  ];
  for (const surface of surfaceOrder) {
    const stored = ids[surface];
    if (stored) {
      add(stored, { kind: "surface", surface, inherited: false });
    } else if (INHERITING_SURFACES.includes(surface) && ids.chat) {
      add(ids.chat, { kind: "surface", surface, inherited: true });
    }
  }

  for (const skill of skills) {
    if (skill.model) add(skill.model, { kind: "skill", skill: skill.name });
  }
  return map;
}

export function sameAsChatLabel(surfaces: Surfaces, models: ModelConfig[]): string {
  const chatId = surfaces.chat.generation.model_id;
  const model = chatId ? models.find((m) => m.id === chatId) : undefined;
  return model ? `Same as Chat (${model.display_name || model.model})` : "Same as Chat";
}
