import { getModelCapabilities } from "$lib/services/capabilities";
import type { ModelCapabilities } from "$lib/types";

let cache = $state<Record<string, ModelCapabilities>>({});
const inflight = new Set<string>();

export function prefetchCapabilities(modelId: string | null): void {
  if (!modelId) return;
  if (cache[modelId]) return;
  if (inflight.has(modelId)) return;
  inflight.add(modelId);
  getModelCapabilities(modelId)
    .then((caps) => {
      cache = { ...cache, [modelId]: caps };
    })
    .catch(() => {})
    .finally(() => {
      inflight.delete(modelId);
    });
}

export function getCachedCapabilities(modelId: string | null): ModelCapabilities | null {
  if (!modelId) return null;
  return cache[modelId] ?? null;
}

export function invalidateCapabilities(modelId: string): void {
  if (!cache[modelId]) return;
  const next = { ...cache };
  delete next[modelId];
  cache = next;
}
