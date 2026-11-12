<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import { Plus, X } from "lucide-svelte";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";
  import TextInput from "$lib/components/shared/ui/TextInput.svelte";
  import { useSaveTracker } from "$lib/stores/saveTracker.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { updateSetting } from "$lib/services/settings";
  import type { WebviewProvider } from "$lib/types";
  import SettingField from "./SettingField.svelte";

  const store = getSettingsStore();
  const tracker = useSaveTracker({
    debounceMs: store.settings?.autosave_debounce_ms ?? 1000,
  });

  type Row = WebviewProvider & { key: number };

  let nextKey = 0;
  let rows = $state<Row[]>(
    untrack(() =>
      (store.settings?.webview_providers ?? []).map((p) => ({ ...p, key: nextKey++ })),
    ),
  );

  onDestroy(() => {
    void tracker.flush();
    tracker.destroy();
  });

  function persist() {
    for (const row of rows) if (!row.id && row.name.trim()) row.id = freshId(row.name);
    const list = rows
      .filter((r) => r.id)
      .map(({ id, name, url }) => ({ id, name, url }));
    return updateSetting("webview_providers", list);
  }

  function slug(name: string): string {
    return name.toLowerCase().replace(/[^a-z0-9_-]/g, "-");
  }

  function freshId(name: string): string {
    const base = slug(name.trim());
    let id = base;
    for (let n = 2; rows.some((r) => r.id === id); n++) id = `${base}-${n}`;
    return id;
  }

  function add() {
    rows.push({ key: nextKey++, id: "", name: "", url: "" });
  }

  function remove(key: number) {
    rows = rows.filter((r) => r.key !== key);
    void tracker.flush(persist);
  }

  function nameChanged(row: Row, name: string) {
    row.name = name;
    tracker.scheduleSave(persist);
  }

  function urlChanged(row: Row, url: string) {
    row.url = url;
    tracker.scheduleSave(persist);
  }
</script>

<div class="providers">
  {#each rows as row (row.key)}
    <div class="row">
      <div class="field">
        <SettingField key="webview_providers[].name" itemId={row.id || undefined}>
          <TextInput
            value={row.name}
            aria-label="Provider name"
            oninput={(e: Event) => nameChanged(row, (e.target as HTMLInputElement).value)}
          />
        </SettingField>
      </div>
      <div class="field">
        <SettingField key="webview_providers[].url" itemId={row.id || undefined}>
          <TextInput
            type="url"
            value={row.url}
            aria-label="Provider URL"
            oninput={(e: Event) => urlChanged(row, (e.target as HTMLInputElement).value)}
          />
        </SettingField>
      </div>
      <div class="remove">
        <ActionIconButton icon={X} label="Remove provider" variant="danger" onclick={() => remove(row.key)} />
      </div>
    </div>
  {/each}
  <div>
    <Button icon={Plus} onclick={add}>Add provider</Button>
  </div>
</div>

<style>
  .providers {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
  }

  .field {
    flex: 1;
    min-width: 0;
  }

  .remove {
    margin-top: var(--space-8);
  }
</style>
