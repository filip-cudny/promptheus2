<script lang="ts">
  import { Plus, X } from "lucide-svelte";
  import { providerIconSvg } from "$lib/icons/providerIcons";
  import { PROMPTHEUS_PROVIDER_ID } from "$lib/services/shellToolbar";
  import type { TabDto } from "$lib/services/tabs";
  import type { WebviewProvider } from "$lib/services/aiWebview";

  const FULL_WIDTH = 196;
  const ICON_WIDTH = 32;
  const GAP = 2;
  const ZONE_GAP = 4;
  const PLUS_WIDTH = 28;
  const MORE_WIDTH = 48;
  const MIN_DRAG = 24;

  let {
    tabs,
    activeId,
    providers,
    onActivate,
    onClose,
    onOpenPlus,
    onOpenOverflow,
  }: {
    tabs: TabDto[];
    activeId: string | null;
    providers: WebviewProvider[];
    onActivate: (id: string) => void;
    onClose: (id: string) => void;
    onOpenPlus: (anchor: HTMLElement) => void;
    onOpenOverflow: (anchor: HTMLElement, hidden: TabDto[]) => void;
  } = $props();

  let zoneEl = $state<HTMLElement | null>(null);
  let zoneWidth = $state(0);

  $effect(() => {
    if (!zoneEl) return;
    const observer = new ResizeObserver(() => {
      zoneWidth = zoneEl?.clientWidth ?? 0;
    });
    observer.observe(zoneEl);
    return () => observer.disconnect();
  });

  function widthOf(visible: TabDto[], collapsed: Set<string>) {
    const tabsWidth = visible.reduce(
      (sum, t) => sum + (collapsed.has(t.id) ? ICON_WIDTH : FULL_WIDTH),
      0,
    );
    return tabsWidth + GAP * Math.max(visible.length - 1, 0);
  }

  let layout = $derived.by(() => {
    const room = zoneWidth - PLUS_WIDTH - MIN_DRAG - 2 * ZONE_GAP;
    const lruFirst = tabs
      .filter((t) => t.id !== activeId)
      .sort((a, b) => b.recency - a.recency);
    const collapsed = new Set<string>();
    for (const t of lruFirst) {
      if (widthOf(tabs, collapsed) <= room) break;
      collapsed.add(t.id);
    }
    if (widthOf(tabs, collapsed) <= room) return { visible: tabs, hidden: [], collapsed };

    const hidden = new Set<string>();
    const shown = () => tabs.filter((t) => !hidden.has(t.id));
    for (const t of lruFirst) {
      if (widthOf(shown(), collapsed) <= room - MORE_WIDTH - ZONE_GAP) break;
      hidden.add(t.id);
    }
    return {
      visible: shown(),
      hidden: tabs.filter((t) => hidden.has(t.id)),
      collapsed,
    };
  });

  function iconFor(tab: TabDto) {
    const id = tab.provider_id ?? PROMPTHEUS_PROVIDER_ID;
    const url = providers.find((p) => p.id === id)?.url;
    return providerIconSvg({ id, url });
  }

  function handleTabKeydown(e: KeyboardEvent, id: string) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onActivate(id);
    }
  }

  let plusEl = $state<HTMLButtonElement | null>(null);
  let moreEl = $state<HTMLButtonElement | null>(null);
</script>

<div class="zone" bind:this={zoneEl} data-tauri-drag-region>
  <div class="tabs">
    {#each layout.visible as tab (tab.id)}
      {@const iconOnly = layout.collapsed.has(tab.id)}
      {@const iconSvg = iconFor(tab)}
      <div
        class="tab"
        class:active={tab.id === activeId}
        class:icon-only={iconOnly}
        class:suspended={tab.kind === "provider" && !tab.live}
        role="tab"
        tabindex="0"
        aria-selected={tab.id === activeId}
        title={tab.title}
        onclick={() => onActivate(tab.id)}
        onkeydown={(e) => handleTabKeydown(e, tab.id)}
      >
        <span class="icon" aria-hidden="true">{#if iconSvg}{@html iconSvg}{/if}</span>
        {#if !iconOnly}
          <span class="label">{tab.title}</span>
        {/if}
        {#if tab.kind === "native" && tab.streaming}
          <span class="dot"></span>
        {/if}
        {#if !iconOnly}
          <button
            type="button"
            class="close"
            aria-label="Close tab"
            onclick={(e) => {
              e.stopPropagation();
              onClose(tab.id);
            }}
          >
            <X size={12} />
          </button>
        {/if}
      </div>
    {/each}
  </div>

  {#if layout.hidden.length > 0}
    <button
      bind:this={moreEl}
      type="button"
      class="more"
      aria-label="More tabs"
      onmousedown={(e) => {
        e.stopPropagation();
        if (moreEl) onOpenOverflow(moreEl, layout.hidden);
      }}
    >
      +{layout.hidden.length}
    </button>
  {/if}

  <button
    bind:this={plusEl}
    type="button"
    class="strip-btn"
    aria-label="New tab"
    onmousedown={(e) => {
      e.stopPropagation();
      if (plusEl) onOpenPlus(plusEl);
    }}
  >
    <Plus size={16} />
  </button>

  <div class="drag-fill" data-tauri-drag-region></div>
</div>

<style>
  .zone {
    flex: 1;
    min-width: 0;
    align-self: stretch;
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
  }

  .tab {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    height: 30px;
    width: 196px;
    flex: 0 0 196px;
    box-sizing: border-box;
    padding: 0 var(--space-4) 0 var(--space-5);
    border-radius: var(--radius-md);
    color: var(--text-muted);
    font-size: var(--font-size-sm);
    cursor: pointer;
  }

  .tab:hover {
    background: var(--surface-overlay-faint);
  }

  .tab.active {
    background: var(--surface-elevated);
    color: var(--text-primary);
    box-shadow: inset 0 0 0 1px var(--border-default);
  }

  .tab.active::after {
    content: "";
    position: absolute;
    left: 10px;
    right: 10px;
    bottom: -5px;
    height: 2px;
    border-radius: 2px;
    background: var(--accent);
  }

  .tab.icon-only {
    width: 32px;
    flex: 0 0 32px;
    padding: 0;
    justify-content: center;
  }

  .tab.suspended .icon {
    opacity: 0.4;
    filter: grayscale(1);
  }

  .tab.suspended .label {
    color: var(--text-disabled);
  }

  .icon {
    width: 15px;
    height: 15px;
    flex: 0 0 15px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .icon :global(svg),
  .icon :global(img) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: contain;
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dot {
    width: 6px;
    height: 6px;
    flex: 0 0 6px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-bg-soft);
  }

  .tab.icon-only .dot {
    position: absolute;
    right: 4px;
    top: 4px;
    box-shadow: 0 0 0 2px var(--surface-base);
  }

  .close {
    appearance: none;
    border: none;
    background: transparent;
    color: var(--text-disabled);
    width: 16px;
    height: 16px;
    flex: 0 0 16px;
    padding: 0;
    border-radius: 4px;
    display: grid;
    place-items: center;
    cursor: pointer;
    opacity: 0;
  }

  .tab.active .close,
  .tab:hover .close {
    opacity: 1;
  }

  .close:hover {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }

  .strip-btn {
    appearance: none;
    border: none;
    background: transparent;
    color: var(--text-muted);
    width: 28px;
    height: 28px;
    flex: 0 0 28px;
    border-radius: var(--radius-md);
    display: grid;
    place-items: center;
    cursor: pointer;
  }

  .strip-btn:hover {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }

  .more {
    appearance: none;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    height: 24px;
    padding: 0 7px;
    font: inherit;
    font-size: var(--font-size-sm);
    cursor: pointer;
  }

  .more:hover {
    background: var(--surface-overlay);
  }

  .drag-fill {
    flex: 1;
    min-width: 24px;
    align-self: stretch;
  }
</style>
