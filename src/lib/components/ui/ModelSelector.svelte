<script lang="ts">
  import { tick } from "svelte";
  import { Check, ChevronDown, Brain } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import {
    supportsReasoning,
    getAvailableReasoningLevels,
    REASONING_LEVEL_LABELS,
    type ReasoningLevel,
  } from "$lib/constants/models";
  import type { ModelConfig } from "$lib/types";

  let {
    models,
    selectedModelId,
    reasoningEffort,
    onModelSelect,
    onReasoningSelect,
    preventDismiss,
    onDropdownToggle,
  }: {
    models: ModelConfig[];
    selectedModelId: string | null;
    reasoningEffort: string | null;
    onModelSelect: (modelId: string) => void;
    onReasoningSelect: (effort: string | null) => void;
    preventDismiss?: { suppress: () => void; resume: () => void };
    onDropdownToggle?: () => void;
  } = $props();

  let modelDropdownOpen = $state(false);
  let reasoningDropdownOpen = $state(false);

  let selectedModel = $derived(
    models.find((m) => m.id === selectedModelId) ?? models[0] ?? null,
  );

  let showReasoning = $derived(
    selectedModel
      ? supportsReasoning(selectedModel.provider, selectedModel.model) ||
        selectedModel.parameters?.reasoning_effort != null
      : false,
  );

  let availableLevels = $derived(
    selectedModel ? getAvailableReasoningLevels(selectedModel.provider) : [],
  );

  let currentLevel = $derived(
    (reasoningEffort as ReasoningLevel | null) ?? "none",
  );

  function toggleModelDropdown(e: MouseEvent) {
    e.stopPropagation();
    if (modelDropdownOpen) {
      closeAll();
    } else {
      reasoningDropdownOpen = false;
      modelDropdownOpen = true;
      preventDismiss?.suppress();
      tick().then(() => onDropdownToggle?.());
    }
  }

  function toggleReasoningDropdown(e: MouseEvent) {
    e.stopPropagation();
    if (reasoningDropdownOpen) {
      closeAll();
    } else {
      modelDropdownOpen = false;
      reasoningDropdownOpen = true;
      preventDismiss?.suppress();
      tick().then(() => onDropdownToggle?.());
    }
  }

  function closeAll() {
    const wasOpen = modelDropdownOpen || reasoningDropdownOpen;
    modelDropdownOpen = false;
    reasoningDropdownOpen = false;
    if (wasOpen) {
      preventDismiss?.resume();
      tick().then(() => onDropdownToggle?.());
    }
  }

  function selectModel(modelId: string) {
    onModelSelect(modelId);
    closeAll();
  }

  function selectReasoning(level: ReasoningLevel) {
    onReasoningSelect(level === "none" ? null : level);
    closeAll();
  }

  function handleClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest(".model-selector")) {
      closeAll();
    }
  }
</script>

<svelte:window onclick={handleClickOutside} />

<div class="model-selector">
  <button class="selector-chip" onclick={toggleModelDropdown} title={selectedModel?.display_name ?? "Select model"}>
    <span class="chip-label">{selectedModel?.display_name ?? "No model"}</span>
    <ChevronDown size={ICON_SIZE.sm} />
  </button>

  {#if showReasoning}
    <div class="reasoning-wrapper">
      <button class="selector-chip reasoning-chip" onclick={toggleReasoningDropdown} title="Reasoning level">
        <Brain size={ICON_SIZE.sm} />
        <span class="chip-label">{REASONING_LEVEL_LABELS[currentLevel]}</span>
        <ChevronDown size={ICON_SIZE.sm} />
      </button>
      {#if reasoningDropdownOpen}
        <div class="dropdown reasoning-dropdown">
          {#each availableLevels as level}
            <button
              class="dropdown-item"
              class:active={level === currentLevel}
              onclick={() => selectReasoning(level)}
            >
              <span class="check-icon">
                {#if level === currentLevel}
                  <Check size={ICON_SIZE.sm} />
                {/if}
              </span>
              <span class="dropdown-label">{REASONING_LEVEL_LABELS[level]}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  {#if modelDropdownOpen}
    <div class="dropdown model-dropdown">
      {#each models as model}
        <button
          class="dropdown-item"
          class:active={model.id === selectedModel?.id}
          onclick={() => selectModel(model.id)}
        >
          <span class="check-icon">
            {#if model.id === selectedModel?.id}
              <Check size={ICON_SIZE.sm} />
            {/if}
          </span>
          <span class="dropdown-label">{model.display_name}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .model-selector {
    display: flex;
    align-items: center;
    gap: 4px;
    position: relative;
  }

  .selector-chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 3px 8px;
    background: #2a2a2a;
    border: 1px solid #3e3e3e;
    border-radius: 10px;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.6);
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
    background: #333;
    color: rgba(255, 255, 255, 0.8);
  }

  .reasoning-wrapper {
    position: relative;
  }

  .reasoning-chip {
    max-width: 120px;
  }

  .chip-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dropdown {
    position: absolute;
    bottom: calc(100% + 4px);
    left: 0;
    z-index: 1000;
    width: 220px;
    background: #252525;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    padding: 4px 0;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }

  .reasoning-dropdown {
    left: 0;
    width: 220px;
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 5px 10px;
    border: none;
    background: transparent;
    color: rgba(255, 255, 255, 0.75);
    font: inherit;
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }

  .dropdown-item:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .dropdown-item.active {
    color: rgba(255, 255, 255, 0.95);
  }

  .check-icon {
    width: 12px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .dropdown-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
