<script lang="ts">
  import { tick } from "svelte";
  import { ChevronDown, Brain } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import {
    REASONING_LEVEL_LABELS,
    type ReasoningLevel,
  } from "$lib/constants/models";
  import type { ModelCapabilities, ModelConfig } from "$lib/types";

  let {
    models,
    selectedModelId,
    reasoningEffort,
    capabilities = null,
    onModelSelect,
    onReasoningSelect,
    preventDismiss,
    onDropdownToggle,
  }: {
    models: ModelConfig[];
    selectedModelId: string | null;
    reasoningEffort: string | null;
    capabilities?: ModelCapabilities | null;
    onModelSelect: (modelId: string) => void;
    onReasoningSelect: (effort: string | null) => void;
    preventDismiss?: { suppress: () => void; resume: () => void };
    onDropdownToggle?: () => void;
  } = $props();

  let modelDropdownOpen = $state(false);
  let reasoningDropdownOpen = $state(false);

  let modelChipEl: HTMLButtonElement | undefined = $state();
  let reasoningChipEl: HTMLButtonElement | undefined = $state();
  let modelDropdownEl: HTMLDivElement | undefined = $state();
  let reasoningDropdownEl: HTMLDivElement | undefined = $state();
  let modelDropdownStyle = $state("");
  let reasoningDropdownStyle = $state("");

  const DROPDOWN_GAP = 4;
  const VIEWPORT_PADDING = 4;

  let selectedModel = $derived(
    models.find((m) => m.id === selectedModelId) ?? models[0] ?? null,
  );

  let currentCapabilities = $derived(capabilities ?? null);

  let showReasoning = $derived(
    currentCapabilities
      ? currentCapabilities.reasoning.kind !== "unsupported"
      : false,
  );

  let availableLevels = $derived.by<ReasoningLevel[]>(() => {
    const reasoning = currentCapabilities?.reasoning;
    if (!reasoning || reasoning.kind === "unsupported") return [];
    if (reasoning.kind === "effort") return ["none", ...reasoning.allowed];
    return ["none", "low", "medium", "high"];
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

  function positionDropdown(
    chipEl: HTMLElement | undefined,
    dropdownEl: HTMLDivElement | undefined,
  ): string {
    if (!chipEl || !dropdownEl) return "";
    const chipRect = chipEl.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    const dropdownWidth = dropdownEl.offsetWidth;
    const naturalHeight = dropdownEl.scrollHeight;

    const spaceAbove = chipRect.top - VIEWPORT_PADDING - DROPDOWN_GAP;
    const spaceBelow = viewportHeight - chipRect.bottom - VIEWPORT_PADDING - DROPDOWN_GAP;
    const openUp = spaceAbove >= naturalHeight || spaceAbove >= spaceBelow;

    const available = Math.max(80, openUp ? spaceAbove : spaceBelow);
    const height = Math.min(naturalHeight, available);

    let top: number;
    if (openUp) {
      top = chipRect.top - DROPDOWN_GAP - height;
    } else {
      top = chipRect.bottom + DROPDOWN_GAP;
    }
    top = Math.max(VIEWPORT_PADDING, top);

    let left = chipRect.right - dropdownWidth;
    left = Math.max(VIEWPORT_PADDING, Math.min(left, viewportWidth - dropdownWidth - VIEWPORT_PADDING));

    return `top: ${top}px; left: ${left}px; max-height: ${height}px;`;
  }

  function refreshDropdownPositions() {
    if (modelDropdownOpen) {
      modelDropdownStyle = positionDropdown(modelChipEl, modelDropdownEl);
    }
    if (reasoningDropdownOpen) {
      reasoningDropdownStyle = positionDropdown(reasoningChipEl, reasoningDropdownEl);
    }
  }

  function toggleModelDropdown(e: MouseEvent) {
    e.stopPropagation();
    if (modelDropdownOpen) {
      closeAll();
    } else {
      reasoningDropdownOpen = false;
      modelDropdownOpen = true;
      preventDismiss?.suppress();
      tick().then(() => {
        modelDropdownStyle = positionDropdown(modelChipEl, modelDropdownEl);
        onDropdownToggle?.();
      });
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
      tick().then(() => {
        reasoningDropdownStyle = positionDropdown(reasoningChipEl, reasoningDropdownEl);
        onDropdownToggle?.();
      });
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

<svelte:window
  onclick={handleClickOutside}
  onresize={refreshDropdownPositions}
  onscroll={refreshDropdownPositions}
/>

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

  {#if showReasoning}
    <div class="reasoning-wrapper">
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
      {#if reasoningDropdownOpen}
        <div
          class="dropdown reasoning-dropdown"
          bind:this={reasoningDropdownEl}
          style={reasoningDropdownStyle}
        >
          {#each availableLevels as level}
            <button
              class="dropdown-item"
              class:active={level === currentLevel}
              onclick={() => selectReasoning(level)}
            >
              <span class="dropdown-label">{REASONING_LEVEL_LABELS[level]}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  {#if modelDropdownOpen}
    <div
      class="dropdown model-dropdown"
      bind:this={modelDropdownEl}
      style={modelDropdownStyle}
    >
      {#each [...groupedModels] as [groupName, groupModels]}
        {#if showGroupHeaders}
          <div class="dropdown-group-label">{groupName}</div>
        {/if}
        {#each groupModels as model}
          <button
            class="dropdown-item"
            class:active={model.id === selectedModel?.id}
            onclick={() => selectModel(model.id)}
          >
            <span class="dropdown-label">{model.display_name}</span>
          </button>
        {/each}
      {/each}
    </div>
  {/if}
</div>

<style>
  .model-selector {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    position: relative;
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
    position: fixed;
    z-index: var(--z-overlay);
    width: 220px;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    padding: var(--space-2) var(--space-0);
    box-shadow: var(--shadow-md);
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .reasoning-dropdown {
    width: 160px;
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: 5px 10px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-md);
    text-align: left;
    cursor: pointer;
  }

  .dropdown-item:hover {
    background: var(--surface-overlay);
  }

  .dropdown-item.active {
    color: var(--accent);
  }

  .dropdown-label {
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
