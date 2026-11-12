import type { Surfaces } from "$lib/types";

export type ToolUse = "chat" | "settings_agent";

export function toolUses(server: string, tool: string, surfaces: Surfaces): ToolUse[] {
  const qualified = `${server}.${tool}`;
  const uses: ToolUse[] = [];
  if (surfaces.chat.generation.enabled_tools.includes(qualified)) uses.push("chat");
  if (surfaces.settings_agent.generation.enabled_tools.includes(qualified)) {
    uses.push("settings_agent");
  }
  return uses;
}
