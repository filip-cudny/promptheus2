import { invoke } from "@tauri-apps/api/core";
import type { Provider } from "$lib/types";

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

export function countTokensDebounced(
  text: string,
  callback: (count: number) => void,
  delay = 300,
): void {
  if (debounceTimer) clearTimeout(debounceTimer);

  if (!text.trim()) {
    callback(0);
    return;
  }

  debounceTimer = setTimeout(async () => {
    try {
      const count = await invoke<number>("count_tokens", { text });
      callback(count);
    } catch {
      callback(0);
    }
  }, delay);
}

const IMAGE_TOKENS: Record<Provider, number> = {
  openai: 765,
  anthropic: 1334,
  gemini: 258,
  elevenlabs: 765,
};

export function estimateImageTokens(provider: Provider): number {
  return IMAGE_TOKENS[provider] ?? 765;
}
