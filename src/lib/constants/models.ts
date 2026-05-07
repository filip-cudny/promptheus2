export type ReasoningLevel = "none" | "minimal" | "low" | "medium" | "high" | "xhigh";

export const REASONING_LEVELS: readonly ReasoningLevel[] = [
  "none",
  "minimal",
  "low",
  "medium",
  "high",
  "xhigh",
];

export const KNOWN_REASONING_EFFORTS: readonly ReasoningLevel[] = REASONING_LEVELS;

export const REASONING_LEVEL_LABELS: Record<ReasoningLevel, string> = {
  none: "None",
  minimal: "Minimal",
  low: "Low",
  medium: "Medium",
  high: "High",
  xhigh: "Extra high",
};
