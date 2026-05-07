import { getModelCapabilities } from "$lib/services/capabilities";
import type { ModelCapabilities, ModelConfig } from "$lib/types";

let cache = $state<Record<string, ModelCapabilities>>({});
const inflight = new Set<string>();

export function prefetchCapabilities(config: ModelConfig | null): void {
  if (!config?.id) return;
  if (cache[config.id]) return;
  if (inflight.has(config.id)) return;
  inflight.add(config.id);
  getModelCapabilities(config.id)
    .then((caps) => {
      cache = { ...cache, [config.id]: caps };
    })
    .catch(() => {})
    .finally(() => {
      inflight.delete(config.id);
    });
}

export function getCachedCapabilities(config: ModelConfig | null): ModelCapabilities | null {
  if (!config?.id) return null;
  return cache[config.id] ?? null;
}

export function invalidateCapabilities(modelId: string): void {
  if (!cache[modelId]) return;
  const next = { ...cache };
  delete next[modelId];
  cache = next;
}

export function clearCapabilitiesCache(): void {
  cache = {};
}
