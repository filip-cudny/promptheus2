<script lang="ts">
  import { Plus, ChevronDown, Star } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import type { ModelConfig, ModelType, SurfaceKind } from "$lib/types";
  import Button from "$lib/components/ui/Button.svelte";

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
        <Button variant="primary" onclick={() => handleAdd("text")}>
          <Plus size={ICON_SIZE.md} /> Add text model
        </Button>
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
    background: var(--surface-sunken);
    border-right: 1px solid var(--border-faint);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-6) var(--space-7);
    border-bottom: 1px solid var(--border-faint);
    flex-shrink: 0;
  }

  h2 {
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--text-muted);
    margin: var(--space-0);
  }

  .add-wrapper {
    position: relative;
  }

  .add-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    background: var(--surface-elevated);
    border: 1px solid var(--border-hard-2);
    border-radius: 5px;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
  }

  .add-btn:hover {
    background: var(--surface-elevated);
  }

  .add-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    background: var(--surface-raised);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    padding: var(--space-2) var(--space-0);
    box-shadow: var(--shadow-md);
    min-width: 180px;
    z-index: var(--z-sticky);
  }

  .add-menu button {
    display: block;
    width: 100%;
    text-align: left;
    padding: var(--space-3) var(--space-6);
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
  }

  .add-menu button:hover {
    background: var(--surface-overlay);
  }

  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4) var(--space-0);
  }

  .empty {
    padding: var(--space-16) var(--space-8);
    text-align: center;
    color: var(--text-muted);
  }

  .empty p {
    margin: var(--space-0) var(--space-0) var(--space-2);
    font-size: var(--font-size-md);
  }

  .empty .muted {
    color: var(--text-disabled);
    margin-bottom: var(--space-7);
  }


  .group-label {
    padding: var(--space-5) var(--space-7) var(--space-2);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    width: 100%;
    padding: var(--space-4) var(--space-7);
    background: transparent;
    border: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    gap: var(--space-4);
  }

  .row:hover:not(.active) {
    background: var(--surface-overlay-faint);
  }

  .row.active {
    background: var(--accent-bg-soft);
  }

  .row-main {
    flex: 1;
    min-width: 0;
  }

  .row-name {
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .default-icon {
    color: var(--warning);
    display: inline-flex;
  }

  .row-model {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
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
    padding: 1px var(--space-3);
    border-radius: var(--radius-xl);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    background: var(--surface-sunken);
    color: var(--text-muted);
    border: 1px solid var(--border-hard);
  }

  .type-chip[data-type="text"] {
    color: var(--accent);
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
