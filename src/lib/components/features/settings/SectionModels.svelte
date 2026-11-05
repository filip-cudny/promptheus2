<script lang="ts">
  import { untrack } from "svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { getSettingsNavigation } from "$lib/stores/settingsNavigation.svelte";
  import ModelList from "./ModelList.svelte";
  import ModelEditor from "./ModelEditor.svelte";
  import CreateModelDialog from "./CreateModelDialog.svelte";
  import type { ModelType } from "$lib/types";

  const store = getSettingsStore();
  const navigation = getSettingsNavigation();

  let selectedId = $state<string | null>(null);

  $effect(() => {
    const ids = store.models.map((m) => m.id);
    if (selectedId && !ids.includes(selectedId)) {
      selectedId = ids[0] ?? null;
    } else if (!selectedId && ids.length > 0) {
      selectedId = ids[0];
    }
  });

  $effect(() => {
    const item = navigation.selectedItem;
    if (item?.list !== "models") return;
    if (untrack(() => store.models.some((m) => m.id === item.id))) selectedId = item.id;
  });

  const selectedModel = $derived(
    selectedId ? store.models.find((m) => m.id === selectedId) ?? null : null,
  );

  let creatingType = $state<ModelType | null>(null);

  function handleSelect(id: string) {
    selectedId = id;
  }
</script>

<div class="section-models">
  <ModelList
    models={store.models}
    selectedId={selectedId}
    surfacesByModel={store.surfacesByModel}
    onSelect={handleSelect}
    onAdd={(type) => (creatingType = type)}
  />
  <div class="editor-pane">
    {#if selectedModel}
      {#key selectedModel.id}
        <ModelEditor
          model={selectedModel}
          surfaces={store.getSurfacesForModel(selectedModel.id)}
          debounceMs={store.settings?.autosave_debounce_ms ?? 1000}
          onDeleted={() => {
            selectedId = null;
          }}
          onDuplicated={(id) => {
            selectedId = id;
          }}
        />
      {/key}
    {:else}
      <div class="empty">
        <h2>No model selected</h2>
        <p>Add a model from the list or pick an existing one to edit.</p>
      </div>
    {/if}
  </div>
</div>

{#if creatingType}
  <CreateModelDialog
    modelType={creatingType}
    onClose={() => (creatingType = null)}
    onCreated={handleSelect}
  />
{/if}

<style>
  .section-models {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .editor-pane {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    background: var(--surface-base);
  }

  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    padding: var(--space-16);
    text-align: center;
  }

  .empty h2 {
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-medium);
    color: var(--text-secondary);
    margin-bottom: var(--space-3);
  }

  .empty p {
    font-size: var(--font-size-md);
  }
</style>
