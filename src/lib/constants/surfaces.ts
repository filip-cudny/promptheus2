import type { SurfaceKind } from "$lib/types";
import type { ModelUse } from "$lib/utils/modelUses";

export const SURFACE_LABELS: Record<SurfaceKind, string> = {
  chat: "Chat",
  quick_actions: "Quick actions",
  title_generation: "Title generation",
  settings_agent: "Settings agent",
  speech_to_text: "Speech to text",
};

export function modelUseLabel(use: ModelUse): string {
  if (use.kind === "skill") return `skill: ${use.skill}`;
  const label = SURFACE_LABELS[use.surface];
  return use.inherited ? `${label} (same as Chat)` : label;
}
