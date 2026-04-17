import type { Provider } from "$lib/types";

export type ReasoningLevel = "none" | "low" | "medium" | "high";

export const REASONING_LEVELS: readonly ReasoningLevel[] = ["none", "low", "medium", "high"];

export const REASONING_LEVEL_LABELS: Record<ReasoningLevel, string> = {
  none: "None",
  low: "Low",
  medium: "Medium",
  high: "High",
};

const OPENAI_REASONING_MODELS: readonly string[] = [
  "o1",
  "o1-mini",
  "o1-preview",
  "o3",
  "o3-mini",
  "o3-pro",
  "o4-mini",
  "o4-mini-deep-research",
  "o3-deep-research",
  "gpt-5",
  "gpt-5-mini",
  "gpt-5-nano",
  "gpt-5.3-codex",
  "gpt-5.4",
  "gpt-5.4-mini",
  "gpt-5.4-nano",
  "gpt-5.4-pro",
];

const ANTHROPIC_REASONING_MODELS: readonly string[] = [
  "claude-opus-4-6",
  "claude-opus-4-5-20251101",
  "claude-opus-4-1-20250805",
  "claude-opus-4-20250514",
  "claude-sonnet-4-6",
  "claude-sonnet-4-5-20250929",
  "claude-sonnet-4-20250514",
  "claude-3-7-sonnet-20250219",
  "claude-haiku-4-5-20251001",
];

const GEMINI_REASONING_MODELS: readonly string[] = [
  "gemini-2.5-flash",
  "gemini-2.5-flash-lite",
  "gemini-2.5-flash-lite-preview",
  "gemini-2.5-flash-preview",
  "gemini-2.5-pro",
  "gemini-3-flash-preview",
  "gemini-3.1-pro",
  "gemini-3.1-flash-lite",
];

const REASONING_MODELS_BY_PROVIDER: Record<Provider, readonly string[]> = {
  openai: OPENAI_REASONING_MODELS,
  anthropic: ANTHROPIC_REASONING_MODELS,
  gemini: GEMINI_REASONING_MODELS,
  elevenlabs: [],
};

export function supportsReasoning(
  provider: Provider | null,
  modelName: string,
): boolean {
  if (!provider) return false;
  const models = REASONING_MODELS_BY_PROVIDER[provider];
  return models?.includes(modelName) ?? false;
}

export function getAvailableReasoningLevels(
  provider: Provider | null,
): ReasoningLevel[] {
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
