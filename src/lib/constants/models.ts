export type ReasoningLevel = "none" | "low" | "medium" | "high";

export const REASONING_LEVELS: readonly ReasoningLevel[] = ["none", "low", "medium", "high"];

export const REASONING_LEVEL_LABELS: Record<ReasoningLevel, string> = {
  none: "None",
  low: "Low",
  medium: "Medium",
  high: "High",
};
