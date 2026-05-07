<script lang="ts">
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import ModelList from "./ModelList.svelte";
  import ModelEditor from "./ModelEditor.svelte";
  import { addModel } from "$lib/services/settings";
  import { generateId } from "$lib/utils/id";
  import type { ModelConfig, ModelType } from "$lib/types";

  const store = getSettingsStore();

  let selectedId = $state<string | null>(null);

  $effect(() => {
    const ids = store.models.map((m) => m.id);
    if (selectedId && !ids.includes(selectedId)) {
      selectedId = ids[0] ?? null;
    } else if (!selectedId && ids.length > 0) {
      selectedId = ids[0];
    }
  });

  const selectedModel = $derived(
    selectedId ? store.models.find((m) => m.id === selectedId) ?? null : null,
  );

  async function handleAdd(modelType: ModelType) {
    const config: ModelConfig = {
      id: generateId(),
      model: "",
      display_name: modelType === "stt" ? "New STT model" : "New text model",
      type: modelType,
      provider: modelType === "text" ? "openai" : null,
      group: null,
      api_key: null,
      base_url: null,
      parameters: null,
      context_window_size: null,
      api_mode: null,
      store: true,
    };
    await addModel(config);
    selectedId = config.id;
  }

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
    onAdd={handleAdd}
  />
  <div class="editor-pane">
    {#if selectedModel}
      {#key selectedModel.id}
        <ModelEditor
          model={selectedModel}
          surfaces={store.getSurfacesForModel(selectedModel.id)}
          debounceMs={store.settings?.number_input_debounce_ms ?? 200}
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
