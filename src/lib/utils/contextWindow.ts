const KNOWN_CONTEXT_WINDOWS: Record<string, number> = {
  "gpt-5.4-pro": 1_050_000,
  "gpt-5.4-mini": 400_000,
  "gpt-5.4": 1_050_000,
  "gpt-5-mini": 128_000,
  "gpt-4o": 128_000,
  "gpt-4o-mini": 128_000,
  "gpt-4.1": 1_047_576,
  "gpt-4.1-mini": 1_047_576,
  "gpt-4.1-nano": 1_047_576,
  "gpt-4-turbo": 128_000,
  "gpt-4": 8_192,
  "gpt-3.5-turbo": 16_385,
  "o1": 200_000,
  "o1-mini": 128_000,
  "o1-pro": 200_000,
  "o3": 200_000,
  "o3-mini": 200_000,
  "o4-mini": 200_000,
  "claude-opus-4.6": 1_000_000,
  "claude-sonnet-4.6": 1_000_000,
  "claude-sonnet-4.5": 200_000,
  "claude-opus-4.5": 200_000,
  "claude-haiku-4.5": 200_000,
  "claude-opus-4": 200_000,
  "claude-sonnet-4": 200_000,
  "claude-3-7-sonnet": 200_000,
  "claude-3-5-sonnet": 200_000,
  "claude-3-5-haiku": 200_000,
  "claude-3-opus": 200_000,
  "claude-3-sonnet": 200_000,
  "claude-3-haiku": 200_000,
  "gemini-2.5-pro": 1_048_576,
  "gemini-2.5-flash": 1_048_576,
  "gemini-2.5-flash-lite": 1_048_576,
  "gemini-2.0-flash": 1_048_576,
  "gemini-2.0-flash-lite": 1_048_576,
  "gemini-1.5-pro": 2_097_152,
  "gemini-1.5-flash": 1_048_576,
};

export function getContextWindowSize(
  model: string,
  configured: number | null,
): number {
  if (configured && configured > 0) return configured;

  const direct = KNOWN_CONTEXT_WINDOWS[model];
  if (direct) return direct;

  for (const [prefix, size] of Object.entries(KNOWN_CONTEXT_WINDOWS)) {
    if (model.startsWith(prefix)) return size;
  }

  return 0;
}

export function formatTokenCount(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
  if (tokens >= 1_000) return `${(tokens / 1_000).toFixed(1)}k`;
  return String(tokens);
}
