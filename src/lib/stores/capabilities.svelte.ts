import { getModelCapabilities } from "$lib/services/capabilities";
import type { ModelCapabilities, ModelConfig } from "$lib/types";

let cache = $state<Record<string, ModelCapabilities>>({});

function cacheKey(provider: string, model: string): string {
  return `${provider}::${model}`;
}

export function prefetchCapabilities(config: ModelConfig | null): void {
  if (!config?.provider) return;
  const key = cacheKey(config.provider, config.model);
  if (cache[key]) return;
  getModelCapabilities(config.provider, config.model).then((caps) => {
    cache = { ...cache, [key]: caps };
  });
}

export function getCachedCapabilities(config: ModelConfig | null): ModelCapabilities | null {
  if (!config?.provider) return null;
  return cache[cacheKey(config.provider, config.model)] ?? null;
}
