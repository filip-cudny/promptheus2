import type { SurfaceKind } from "$lib/types";

export const SURFACE_ORDER: SurfaceKind[] = [
  "chat",
  "quick_actions",
  "title_generation",
  "settings_agent",
  "speech_to_text",
];

export const SURFACE_LABELS: Record<SurfaceKind, string> = {
  chat: "chat",
  quick_actions: "quick actions",
  title_generation: "title generation",
  settings_agent: "settings agent",
  speech_to_text: "speech to text",
};

export function formatSurfaceList(surfaces: SurfaceKind[]): string {
  return surfaces.map((s) => SURFACE_LABELS[s]).join(", ");
}
