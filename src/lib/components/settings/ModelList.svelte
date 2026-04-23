<script lang="ts">
  import { Plus, ChevronDown, Star } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import type { ModelConfig, ModelType, SurfaceKind } from "$lib/types";

  let {
    models,
    selectedId,
    surfaceModelIds,
    onSelect,
    onAdd,
  }: {
    models: ModelConfig[];
    selectedId: string | null;
    surfaceModelIds: Record<SurfaceKind, string | null>;
    onSelect: (id: string) => void;
    onAdd: (type: ModelType) => void;
  } = $props();

  let addMenuOpen = $state(false);

  const referencedIds = $derived.by(() => {
    const ids = new Set<string>();
    Object.values(surfaceModelIds).forEach((v) => {
      if (v) ids.add(v);
    });
    return ids;
  });

  const groups = $derived.by(() => {
    const map = new Map<string, ModelConfig[]>();
    for (const m of models) {
      const key = m.group ?? "Ungrouped";
      const list = map.get(key) ?? [];
      list.push(m);
      map.set(key, list);
    }
    for (const list of map.values()) {
      list.sort((a, b) => a.display_name.localeCompare(b.display_name));
    }
    return [...map.entries()].sort(([a], [b]) => {
      if (a === "Ungrouped") return 1;
      if (b === "Ungrouped") return -1;
      return a.localeCompare(b);
    });
  });

  function handleAdd(type: ModelType) {
    addMenuOpen = false;
    onAdd(type);
  }

  function handleClickOutside(e: MouseEvent) {
    if (!addMenuOpen) return;
    const target = e.target as HTMLElement;
    if (!target.closest(".add-wrapper")) addMenuOpen = false;
  }
</script>

<svelte:window onclick={handleClickOutside} />

<aside class="model-list">
  <header>
    <h2>Models</h2>
    <div class="add-wrapper">
      <button
        class="add-btn"
        onclick={(e) => {
          e.stopPropagation();
          addMenuOpen = !addMenuOpen;
        }}
        title="Add model"
      >
        <Plus size={ICON_SIZE.md} />
        <span>Add</span>
        <ChevronDown size={ICON_SIZE.sm} />
      </button>
      {#if addMenuOpen}
        <div class="add-menu">
          <button onclick={() => handleAdd("text")}>Text model</button>
          <button onclick={() => handleAdd("stt")}>Speech-to-text model</button>
        </div>
      {/if}
    </div>
  </header>

  <div class="scroll">
    {#if models.length === 0}
      <div class="empty">
        <p>No models configured.</p>
        <p class="muted">Add your first model to get started.</p>
        <button class="primary-cta" onclick={() => handleAdd("text")}>
          <Plus size={ICON_SIZE.md} /> Add text model
        </button>
      </div>
    {:else}
      {#each groups as [groupName, groupModels] (groupName)}
        <div class="group-label">{groupName}</div>
        {#each groupModels as model (model.id)}
          <button
            class="row"
            class:active={model.id === selectedId}
            onclick={() => onSelect(model.id)}
          >
            <div class="row-main">
              <div class="row-name">
                {model.display_name || "(unnamed)"}
                {#if referencedIds.has(model.id)}
                  <span class="default-icon" title="Referenced by a surface">
                    <Star size={10} fill="currentColor" />
                  </span>
                {/if}
              </div>
              <div class="row-model">{model.model || "—"}</div>
            </div>
            <div class="row-chips">
              <span class="chip type-chip" data-type={model.type}>
                {model.type === "stt" ? "STT" : "Text"}
              </span>
              {#if model.provider}
                <span class="chip provider-chip">{model.provider}</span>
              {/if}
            </div>
          </button>
        {/each}
      {/each}
    {/if}
  </div>
</aside>

<style>
  .model-list {
    width: 280px;
    flex-shrink: 0;
    background: #1c1c1c;
    border-right: 1px solid rgba(255, 255, 255, 0.06);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    flex-shrink: 0;
  }

  h2 {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: rgba(255, 255, 255, 0.45);
    margin: 0;
  }

  .add-wrapper {
    position: relative;
  }

  .add-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: #2a2a2a;
    border: 1px solid #3e3e3e;
    border-radius: 5px;
    color: rgba(255, 255, 255, 0.78);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .add-btn:hover {
    background: #333;
  }

  .add-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    background: #252525;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    padding: 4px 0;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    min-width: 180px;
    z-index: 10;
  }

  .add-menu button {
    display: block;
    width: 100%;
    text-align: left;
    padding: 6px 12px;
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.78);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .add-menu button:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }

  .empty {
    padding: 32px 16px;
    text-align: center;
    color: rgba(255, 255, 255, 0.55);
  }

  .empty p {
    margin: 0 0 4px;
    font-size: 12px;
  }

  .empty .muted {
    color: rgba(255, 255, 255, 0.35);
    margin-bottom: 14px;
  }

  .primary-cta {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 6px 12px;
    background: rgba(91, 141, 217, 0.15);
    color: #8db3ee;
    border: 1px solid rgba(91, 141, 217, 0.4);
    border-radius: 5px;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .primary-cta:hover {
    background: rgba(91, 141, 217, 0.22);
  }

  .group-label {
    padding: 10px 14px 4px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.32);
  }

  .row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    width: 100%;
    padding: 8px 14px;
    background: transparent;
    border: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    gap: 8px;
  }

  .row:hover:not(.active) {
    background: rgba(255, 255, 255, 0.04);
  }

  .row.active {
    background: rgba(91, 141, 217, 0.15);
  }

  .row-main {
    flex: 1;
    min-width: 0;
  }

  .row-name {
    font-size: 13px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.92);
    display: flex;
    align-items: center;
    gap: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .default-icon {
    color: #d9b34a;
    display: inline-flex;
  }

  .row-model {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    margin-top: 1px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-chips {
    display: flex;
    flex-direction: column;
    gap: 3px;
    align-items: flex-end;
    flex-shrink: 0;
  }

  .chip {
    display: inline-flex;
    padding: 1px 6px;
    border-radius: 8px;
    font-size: 10px;
    font-weight: 500;
    background: #2e2e2e;
    color: rgba(255, 255, 255, 0.55);
    border: 1px solid #3a3a3a;
  }

  .type-chip[data-type="text"] {
    color: #6da6ee;
    border-color: rgba(109, 166, 238, 0.3);
  }

  .type-chip[data-type="stt"] {
    color: #c084d8;
    border-color: rgba(192, 132, 216, 0.3);
  }

  .provider-chip {
    text-transform: capitalize;
  }
</style>
