<script lang="ts">
  import { onDestroy, onMount, tick, untrack } from "svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { getSkillsStore } from "$lib/stores/skills.svelte";
  import { getSettingsNavigation } from "$lib/stores/settingsNavigation.svelte";
  import ModelList from "./ModelList.svelte";
  import ModelEditor from "./ModelEditor.svelte";
  import ModelAssignments from "./ModelAssignments.svelte";
  import CreateModelDialog from "./CreateModelDialog.svelte";
  import type { ModelType, SurfaceKind } from "$lib/types";

  const HIGHLIGHT_MS = 2000;

  const store = getSettingsStore();
  const navigation = getSettingsNavigation();

  let selectedId = $state<string | null>(null);

  onMount(() => void getSkillsStore().init());

  $effect(() => {
    if (selectedId && !store.models.some((m) => m.id === selectedId)) selectedId = null;
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

  $effect(() => {
    const type = navigation.createModelType;
    if (!type) return;
    creatingType = type;
    navigation.clearCreateModelType();
  });

  let highlightedSurface = $state<SurfaceKind | null>(null);
  let highlightTimer: ReturnType<typeof setTimeout> | null = null;

  onDestroy(() => {
    if (highlightTimer) clearTimeout(highlightTimer);
  });

  function handleSelect(id: string) {
    selectedId = id;
  }

  async function showUse(surface: SurfaceKind) {
    selectedId = null;
    if (highlightTimer) clearTimeout(highlightTimer);
    highlightedSurface = surface;
    highlightTimer = setTimeout(() => {
      highlightedSurface = null;
      highlightTimer = null;
    }, HIGHLIGHT_MS);
    await tick();
    document.getElementById(`assignment-${surface}`)?.scrollIntoView({ block: "center" });
  }
</script>

<div class="section-models">
  <ModelList
    models={store.models}
    selectedId={selectedId}
    assignmentsSelected={selectedId === null}
    usesByModel={store.usesByModel}
    onSelect={handleSelect}
    onSelectAssignments={() => (selectedId = null)}
    onShowUse={showUse}
    onAdd={(type) => (creatingType = type)}
  />
  <div class="editor-pane">
    {#if selectedModel}
      {#key selectedModel.id}
        <ModelEditor
          model={selectedModel}
          uses={store.getUsesForModel(selectedModel.id)}
          onShowUse={showUse}
          onShowAssignments={() => (selectedId = null)}
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
      <ModelAssignments highlighted={highlightedSurface} />
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

  .section-models > :global(.model-list) {
    margin-left: var(--column-gutter);
  }

  .editor-pane {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding-right: var(--column-gutter);
    background: var(--surface-base);
  }
</style>
