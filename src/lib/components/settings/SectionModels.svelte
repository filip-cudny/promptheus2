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
    surfaceModelIds={store.surfaceModelIds}
    onSelect={handleSelect}
    onAdd={handleAdd}
  />
  <div class="editor-pane">
    {#if selectedModel}
      {#key selectedModel.id}
        <ModelEditor
          model={selectedModel}
          referencedSurface={store.isModelReferencedBySurface(selectedModel.id)}
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
    background: #1e1e1e;
  }

  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.45);
    padding: 32px;
    text-align: center;
  }

  .empty h2 {
    font-size: 14px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.7);
    margin-bottom: 6px;
  }

  .empty p {
    font-size: 12px;
  }
</style>
