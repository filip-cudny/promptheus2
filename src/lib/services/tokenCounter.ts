import { invoke } from "@tauri-apps/api/core";
import type { Provider } from "$lib/types";

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

export function countTokensDebounced(
  text: string,
  provider: Provider,
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
      const count = await invoke<number>("count_tokens", { text, provider });
      callback(count);
    } catch {
      callback(0);
    }
  }, delay);
}

export async function countTokens(
  text: string,
  provider: Provider,
): Promise<number> {
  if (!text.trim()) return 0;
  try {
    return await invoke<number>("count_tokens", { text, provider });
  } catch {
    return 0;
  }
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
