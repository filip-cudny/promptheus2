<script lang="ts">
  import type { Snippet } from "svelte";
  import { warn as logWarn } from "@tauri-apps/plugin-log";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import {
    catalogPath,
    getSettingsCatalogStore,
  } from "$lib/stores/settingsCatalog.svelte";
  import FormRow from "$lib/components/shared/ui/FormRow.svelte";

  let {
    key,
    itemId,
    children,
  }: {
    key: string;
    itemId?: string;
    children: Snippet;
  } = $props();

  const store = getSettingsStore();
  const catalog = getSettingsCatalogStore();

  const entry = $derived(catalog.entry(key));
  const restartNote = $derived(
    entry?.applies === "after_restart" ? "Applies after restart." : "",
  );
  const hint = $derived(
    [entry?.description, restartNote].filter(Boolean).join(" ") || undefined,
  );
  const error = $derived(
    Object.entries(store.fieldErrors).find(
      ([concrete]) =>
        catalogPath(concrete) === key &&
        (itemId === undefined || concrete.includes(`[${itemId}]`)),
    )?.[1],
  );

  $effect(() => {
    if (catalog.loaded && !entry) logWarn(`settings catalog has no entry for ${key}`);
  });
</script>

<div id={`setting-${key}`}>
  <FormRow label={entry?.label ?? key} {hint} {error}>
    {@render children()}
  </FormRow>
</div>
