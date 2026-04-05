import type { Provider } from "$lib/types";

export type ReasoningLevel = "none" | "low" | "medium" | "high";

export const REASONING_LEVELS: readonly ReasoningLevel[] = ["none", "low", "medium", "high"];

export const REASONING_LEVEL_LABELS: Record<ReasoningLevel, string> = {
  none: "None",
  low: "Low",
  medium: "Med",
  high: "High",
};

const OPENAI_REASONING_PREFIXES = ["o1", "o3", "o4"];

export function supportsReasoning(provider: Provider, modelName: string): boolean {
  switch (provider) {
    case "openai":
      return OPENAI_REASONING_PREFIXES.some(
        (prefix) => modelName === prefix || modelName.startsWith(`${prefix}-`),
      );
    case "anthropic":
      return modelName.includes("claude") && (modelName.includes("3.5") || modelName.includes("3-5") || modelName.includes("4"));
    case "gemini":
      return modelName.includes("thinking") || modelName.includes("2.5");
    default:
      return false;
  }
}

export function getAvailableReasoningLevels(provider: Provider): ReasoningLevel[] {
  switch (provider) {
    case "openai":
      return ["none", "low", "medium", "high"];
    case "anthropic":
      return ["none", "low", "medium", "high"];
    case "gemini":
      return ["none", "low", "medium", "high"];
    default:
      return [];
  }
}
