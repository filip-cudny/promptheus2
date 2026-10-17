<script lang="ts">
  import { ChevronDown, Brain } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import {
    REASONING_LEVEL_LABELS,
    type ReasoningLevel,
  } from "$lib/constants/models";
  import FloatingPanel from "$lib/components/shared/ui/FloatingPanel.svelte";
  import MenuList from "$lib/components/shared/ui/MenuList.svelte";
  import MenuItem from "$lib/components/shared/ui/MenuItem.svelte";
  import type { ModelCapabilities, ModelConfig } from "$lib/types";

  let {
    models,
    selectedModelId,
    reasoningEffort,
    capabilities = null,
    onModelSelect,
    onReasoningSelect,
    preventDismiss,
  }: {
    models: ModelConfig[];
    selectedModelId: string | null;
    reasoningEffort: string | null;
    capabilities?: ModelCapabilities | null;
    onModelSelect: (modelId: string) => void;
    onReasoningSelect: (effort: string | null) => void;
    preventDismiss?: { suppress: () => void; resume: () => void };
  } = $props();

  let modelDropdownOpen = $state(false);
  let reasoningDropdownOpen = $state(false);

  let modelChipEl: HTMLButtonElement | undefined = $state();
  let reasoningChipEl: HTMLButtonElement | undefined = $state();

  let selectedModel = $derived(
    models.find((m) => m.id === selectedModelId) ?? models[0] ?? null,
  );

  let currentCapabilities = $derived(capabilities ?? null);
  let reasoningKind = $derived(
    currentCapabilities?.reasoning.kind ?? "unsupported",
  );
  let showReasoning = $derived(reasoningKind !== "unsupported");

  let availableLevels = $derived.by<ReasoningLevel[]>(() => {
    const reasoning = currentCapabilities?.reasoning;
    if (!reasoning || reasoning.kind !== "effort") return [];
    const allowed = reasoning.allowed;
    return allowed.includes("none") ? allowed : ["none", ...allowed];
  });

  let currentLevel = $derived(
    (reasoningEffort as ReasoningLevel | null) ?? "none",
  );

  let groupedModels = $derived.by(() => {
    const groups = new Map<string, ModelConfig[]>();
    for (const model of models) {
      const key = model.group ?? model.provider ?? "other";
      const list = groups.get(key);
      if (list) list.push(model);
      else groups.set(key, [model]);
    }
    return groups;
  });

  let showGroupHeaders = $derived(groupedModels.size > 1);

  function toggleModelDropdown() {
    if (modelDropdownOpen) {
      closeAll();
    } else {
      reasoningDropdownOpen = false;
      modelDropdownOpen = true;
      preventDismiss?.suppress();
    }
  }

  function toggleReasoningDropdown() {
    if (reasoningDropdownOpen) {
      closeAll();
    } else {
      modelDropdownOpen = false;
      reasoningDropdownOpen = true;
      preventDismiss?.suppress();
    }
  }

  function closeAll() {
    const wasOpen = modelDropdownOpen || reasoningDropdownOpen;
    modelDropdownOpen = false;
    reasoningDropdownOpen = false;
    if (wasOpen) preventDismiss?.resume();
  }

  function selectModel(modelId: string) {
    onModelSelect(modelId);
    closeAll();
  }

  function selectReasoning(level: ReasoningLevel) {
    onReasoningSelect(level);
    closeAll();
  }
</script>

<div class="model-selector">
  <button
    class="selector-chip"
    bind:this={modelChipEl}
    onclick={toggleModelDropdown}
    title={selectedModel?.display_name ?? "Select model"}
  >
    <span class="chip-label">{selectedModel?.display_name ?? "No model"}</span>
    <ChevronDown size={ICON_SIZE.sm} />
  </button>
  <FloatingPanel
    visible={modelDropdownOpen}
    anchorEl={modelChipEl}
    position="above"
    align="end"
    fitContent
    onclose={closeAll}
  >
    <MenuList>
      {#each [...groupedModels] as [groupName, groupModels]}
        {#if showGroupHeaders}
          <div class="dropdown-group-label">{groupName}</div>
        {/if}
        {#each groupModels as model}
          <MenuItem
            label={model.display_name}
            active={model.id === selectedModel?.id}
            onclick={() => selectModel(model.id)}
          />
        {/each}
      {/each}
    </MenuList>
  </FloatingPanel>

  {#if showReasoning && reasoningKind === "effort"}
    <button
      class="selector-chip reasoning-chip"
      bind:this={reasoningChipEl}
      onclick={toggleReasoningDropdown}
      title="Reasoning level"
    >
      <Brain size={ICON_SIZE.sm} />
      <span class="chip-label">{REASONING_LEVEL_LABELS[currentLevel]}</span>
      <ChevronDown size={ICON_SIZE.sm} />
    </button>
    <FloatingPanel
      visible={reasoningDropdownOpen}
      anchorEl={reasoningChipEl}
      position="above"
      align="end"
      fitContent
      onclose={closeAll}
    >
      <MenuList>
        {#each availableLevels as level}
          <MenuItem
            label={REASONING_LEVEL_LABELS[level]}
            active={level === currentLevel}
            onclick={() => selectReasoning(level)}
          />
        {/each}
      </MenuList>
    </FloatingPanel>
  {/if}
</div>

<style>
  .model-selector {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .selector-chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 3px 8px;
    background: var(--surface-elevated);
    border: 1px solid var(--border-hard-2);
    border-radius: 10px;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    cursor: pointer;
    white-space: nowrap;
    font-family: inherit;
    max-width: 140px;
  }

  .selector-chip :global(svg) {
    flex-shrink: 0;
    display: block;
  }

  .selector-chip:hover {
    background: var(--surface-elevated);
    color: var(--text-secondary);
  }

  .reasoning-chip {
    max-width: 120px;
  }

  .chip-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dropdown-group-label {
    padding: var(--space-3) var(--space-5) var(--space-1);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: var(--tracking-label);
    color: var(--text-disabled);
  }

  .dropdown-group-label:not(:first-child) {
    margin-top: 4px;
    border-top: 1px solid var(--border-faint);
    padding-top: 8px;
  }
</style>
