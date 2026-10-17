<script lang="ts">
  import { RefreshCw } from "lucide-svelte";
  import CommandPalette from "$lib/components/shared/ui/CommandPalette.svelte";
  import KbdHint from "$lib/components/shared/ui/KbdHint.svelte";
  import { providerIconSvg } from "$lib/icons/providerIcons";
  import { PROMPTHEUS_PROVIDER_ID } from "$lib/services/shellToolbar";
  import { useListSelection } from "$lib/components/shared/drivers/useListSelection.svelte";
  import { SHORTCUTS, matches } from "$lib/shortcuts";
  import type { TabEntry } from "./drivers/usePaletteIpc.svelte";

  type ProviderEntry = { kind: "provider"; id: string; name: string; url?: string };
  type TabSwitchEntry = {
    kind: "tab";
    id: string;
    name: string;
    tabId: string;
    providerId: string | null;
    isCurrent: boolean;
  };
  type ActionEntry = { kind: "action"; id: string; name: string };
  type PaletteEntry = ProviderEntry | TabSwitchEntry | ActionEntry;

  const ACTION_RELOAD_ID = "action:reload-active";

  let {
    visible,
    activeId,
    webviewProviders,
    openWindows,
    currentTitle,
    inputRef = $bindable(undefined),
    onDismiss,
    onReloadActive,
    onFocusWindow,
  }: {
    visible: boolean;
    activeId: string;
    webviewProviders: { id: string; name: string; url?: string | null }[];
    openWindows: TabEntry[];
    currentTitle: string;
    inputRef?: HTMLInputElement | undefined;
    onDismiss: (selectedId: string | null) => void;
    onReloadActive: () => void;
    onFocusWindow: (tabId: string) => void;
  } = $props();

  let query = $state("");

  let providers = $derived<ProviderEntry[]>([
    { kind: "provider", id: PROMPTHEUS_PROVIDER_ID, name: "Promptheus" },
    ...webviewProviders.map<ProviderEntry>((p) => ({
      kind: "provider",
      id: p.id,
      name: p.name,
      url: p.url ?? undefined,
    })),
  ]);

  let activeName = $derived(providers.find((p) => p.id === activeId)?.name ?? "active provider");

  let tabEntries = $derived<TabSwitchEntry[]>(
    openWindows.map((tab) => ({
      kind: "tab",
      id: `tab:${tab.id}`,
      name: tab.title,
      tabId: tab.id,
      providerId: tab.kind === "provider" ? tab.provider_id : PROMPTHEUS_PROVIDER_ID,
      isCurrent: tab.is_current,
    })),
  );

  let actions = $derived<ActionEntry[]>([
    { kind: "action", id: ACTION_RELOAD_ID, name: `Reload ${activeName}` },
  ]);

  let entries = $derived<PaletteEntry[]>([...providers, ...tabEntries, ...actions]);

  let filtered = $derived.by<PaletteEntry[]>(() => {
    const q = query.trim().toLowerCase();
    if (!q) return entries;
    return entries.filter((e) => e.name.toLowerCase().includes(q));
  });

  let headerLabel = $derived((currentTitle ?? "").trim());

  const selection = useListSelection({
    count: () => filtered.length,
    active: () => visible,
    onEscape: () => onDismiss(null),
    onEnter: (i) => {
      const entry = filtered[i];
      if (entry) selectEntry(entry);
    },
  });

  $effect(() => {
    if (visible) {
      query = "";
      selection.index = 0;
    }
  });

  function selectEntry(entry: PaletteEntry) {
    if (entry.kind === "action") {
      if (entry.id === ACTION_RELOAD_ID) onReloadActive();
      return;
    }
    if (entry.kind === "tab") {
      if (entry.isCurrent) {
        onDismiss(null);
        return;
      }
      onFocusWindow(entry.tabId);
      return;
    }
    onDismiss(entry.id);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (matches(e, SHORTCUTS.reloadActive)) {
      e.preventDefault();
      onReloadActive();
      return;
    }
    if (selection.handleKeydown(e)) return;
    if (matches(e, SHORTCUTS.openPalette)) {
      e.preventDefault();
      onDismiss(null);
      return;
    }
  }

  function isFirstOfKind(entry: PaletteEntry, i: number): boolean {
    if (i === 0) return false;
    return filtered[i - 1].kind !== entry.kind;
  }

  function tabIconProvider(entry: TabSwitchEntry) {
    const provider = webviewProviders.find((p) => p.id === entry.providerId);
    return { id: entry.providerId ?? "", url: provider?.url ?? undefined };
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<CommandPalette
  open={visible}
  onClose={() => onDismiss(null)}
  bind:query
  bind:inputRef
  placeholder="Search providers, tabs, and actions..."
  variant="window"
  bodyMaxHeight="360px"
>
  {#snippet headerExtras()}
    {#if headerLabel}
      <span class="palette-context" title={headerLabel}>{headerLabel}</span>
    {/if}
  {/snippet}
  {#snippet body()}
    {#each filtered as entry, i (entry.id)}
      {#if isFirstOfKind(entry, i)}
        <div class="palette-divider" role="separator"></div>
      {/if}
      <button
        bind:this={selection.itemEls[i]}
        type="button"
        role="option"
        aria-selected={i === selection.index}
        class="palette-item"
        class:highlight={i === selection.index}
        onmouseenter={() => (selection.index = i)}
        onclick={() => selectEntry(entry)}
      >
        {#if entry.kind === "provider"}
          {@const iconSvg = providerIconSvg(entry)}
          <span class="palette-item-icon" aria-hidden="true">
            {#if iconSvg}{@html iconSvg}{/if}
          </span>
          <span class="palette-item-name">{entry.name}</span>
          {#if entry.id === activeId}
            <span class="badge">active</span>
          {/if}
        {:else if entry.kind === "tab"}
          {@const iconSvg = providerIconSvg(tabIconProvider(entry))}
          <span class="palette-item-icon" class:muted={entry.isCurrent} aria-hidden="true">
            {#if iconSvg}
              <span class="tab-icon-base">{@html iconSvg}</span>
            {:else}
              <span class="tab-icon-base placeholder"></span>
            {/if}
          </span>
          <span class="palette-item-name" class:muted-text={entry.isCurrent}>{entry.name}</span>
          {#if entry.isCurrent}
            <span class="badge">here</span>
          {:else}
            <span class="badge subtle">switch</span>
          {/if}
        {:else}
          <span class="palette-item-icon muted" aria-hidden="true">
            <RefreshCw size={14} />
          </span>
          <span class="palette-item-name">{entry.name}</span>
        {/if}
      </button>
    {:else}
      <div class="palette-empty">no matches</div>
    {/each}
  {/snippet}
  {#snippet footer()}
    <span><KbdHint keys={["↑↓"]} /> / <KbdHint keys={["⌃JK"]} /> Navigate</span>
    <span><KbdHint keys={["↵"]} /> Select</span>
    <span><KbdHint keys={["⌘R"]} /> Reload</span>
    <span><KbdHint keys={["esc"]} /> Close</span>
  {/snippet}
</CommandPalette>

<style>
  :global(.palette-item-icon.muted) {
    opacity: 0.45;
  }

  :global(.palette-item.highlight .palette-item-icon.muted) {
    opacity: 0.75;
  }

  .palette-context {
    flex-shrink: 0;
    max-width: 220px;
    margin: 0 var(--space-7) 0 var(--space-2);
    padding: 2px 8px;
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-sm);
    background: var(--surface-overlay-faint);
    color: var(--text-muted);
    font-size: var(--font-size-xs);
    font-weight: 500;
    letter-spacing: 0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .badge {
    font-size: var(--font-size-xs);
    text-transform: uppercase;
    color: var(--text-muted);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm);
    padding: 1px 5px;
  }

  .badge.subtle {
    border-color: transparent;
    background: var(--surface-overlay-faint);
    opacity: 0.55;
  }

  :global(.palette-item.highlight .badge.subtle) {
    opacity: 0.85;
  }

  .muted-text {
    opacity: 0.55;
  }

  :global(.palette-item.highlight .muted-text) {
    opacity: 0.8;
  }

  .tab-icon-base {
    width: 100%;
    height: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  :global(.tab-icon-base svg),
  :global(.tab-icon-base img) {
    width: 100%;
    height: 100%;
    display: block;
  }

  .tab-icon-base.placeholder {
    background: var(--surface-overlay-faint);
    border-radius: var(--radius-xs);
  }
</style>
