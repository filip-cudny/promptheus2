import { getWebviewProviders, type WebviewProvider } from "$lib/services/aiWebview";
import { PROMPTHEUS_PROVIDER_ID } from "$lib/services/shellToolbar";
import { onSettingsChanged } from "$lib/services/events";

export type ProviderEntry = { id: string; name: string; url?: string | null };

export function createWebviewProvidersStore() {
  let providers = $state<WebviewProvider[]>([]);

  const providerEntries = $derived<ProviderEntry[]>([
    { id: PROMPTHEUS_PROVIDER_ID, name: "Promptheus" },
    ...providers.map((p) => ({ id: p.id, name: p.name, url: p.url })),
  ]);

  async function refresh() {
    try {
      providers = await getWebviewProviders();
    } catch (e) {
      console.error("getWebviewProviders failed", e);
    }
  }

  async function init(): Promise<() => void> {
    await refresh();
    return await onSettingsChanged(refresh);
  }

  return {
    get providers() { return providers; },
    get providerEntries() { return providerEntries; },
    init,
  };
}

export type WebviewProvidersStore = ReturnType<typeof createWebviewProvidersStore>;
