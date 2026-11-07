import type { SurfaceKind } from "$lib/types";

export const SURFACE_ORDER: SurfaceKind[] = [
  "chat",
  "quick_actions",
  "title_generation",
  "settings_agent",
  "speech_to_text",
];

export const SURFACE_LABELS: Record<SurfaceKind, string> = {
  chat: "Chat",
  quick_actions: "Quick actions",
  title_generation: "Title generation",
  settings_agent: "Settings agent",
  speech_to_text: "Speech to text",
};

export function formatSurfaceList(surfaces: SurfaceKind[]): string {
  return surfaces.map((s) => SURFACE_LABELS[s]).join(", ");
}
