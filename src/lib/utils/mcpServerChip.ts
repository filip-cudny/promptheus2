export type McpServerRegistryState = "running" | "error" | "off";

export type McpServerChip =
  | "running"
  | "stops after restart"
  | "error"
  | "starts after restart"
  | "off";

export function serverChip(state: McpServerRegistryState | null, disabled: boolean): McpServerChip {
  if (disabled) return state === "running" ? "stops after restart" : "off";
  if (state === "running") return "running";
  if (state === "error") return "error";
  return "starts after restart";
}
