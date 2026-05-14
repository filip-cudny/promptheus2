<script lang="ts">
  import { AppWindow, RefreshCw } from "lucide-svelte";
  import CommandPalette from "$lib/components/shared/ui/CommandPalette.svelte";
  import KbdHint from "$lib/components/shared/ui/KbdHint.svelte";
  import { providerIconSvg } from "$lib/icons/providerIcons";
  import { PROMPTHEUS_PROVIDER_ID } from "$lib/services/shellToolbar";
  import { handleListNavKey } from "$lib/utils/listNavigation";
  import { SHORTCUTS, matches } from "$lib/shortcuts";
  import type { WindowEntry } from "./drivers/usePaletteIpc.svelte";

  type ProviderEntry = { kind: "provider"; id: string; name: string; url?: string };
  type WindowSwitchEntry = {
    kind: "window";
    id: string;
    name: string;
    hostLabel: string;
    providerId: string | null;
    providerUrl: string | null;
    isCurrent: boolean;
  };
  type ActionEntry = { kind: "action"; id: string; name: string };
  type PaletteEntry = ProviderEntry | WindowSwitchEntry | ActionEntry;

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
    openWindows: WindowEntry[];
    currentTitle: string;
    inputRef?: HTMLInputElement | undefined;
    onDismiss: (selectedId: string | null) => void;
    onReloadActive: () => void;
    onFocusWindow: (label: string) => void;
  } = $props();

  let query = $state("");
  let index = $state(0);
  let itemEls: (HTMLElement | null)[] = $state([]);

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

  let windowEntries = $derived<WindowSwitchEntry[]>(
    openWindows.map((w) => ({
      kind: "window",
      id: `window:${w.host_label}`,
      name: w.title,
      hostLabel: w.host_label,
      providerId: w.kind === "ai_provider" ? w.provider_id : PROMPTHEUS_PROVIDER_ID,
      providerUrl: w.provider_url,
      isCurrent: w.is_current,
    })),
  );

  let actions = $derived<ActionEntry[]>([
    { kind: "action", id: ACTION_RELOAD_ID, name: `Reload ${activeName}` },
  ]);

  let entries = $derived<PaletteEntry[]>([...providers, ...windowEntries, ...actions]);

  let filtered = $derived.by<PaletteEntry[]>(() => {
    const q = query.trim().toLowerCase();
    if (!q) return entries;
    return entries.filter((e) => e.name.toLowerCase().includes(q));
  });

  let headerLabel = $derived((currentTitle ?? "").trim());

  $effect(() => {
    if (visible) {
      query = "";
      index = 0;
    }
  });

  $effect(() => {
    const _ = filtered.length;
    if (index >= filtered.length) {
      index = Math.max(0, filtered.length - 1);
    }
  });

  $effect(() => {
    if (!visible) return;
    const el = itemEls[index];
    if (el) el.scrollIntoView({ block: "nearest" });
  });

  function selectEntry(entry: PaletteEntry) {
    if (entry.kind === "action") {
      if (entry.id === ACTION_RELOAD_ID) onReloadActive();
      return;
    }
    if (entry.kind === "window") {
      if (entry.isCurrent) {
        onDismiss(null);
        return;
      }
      onFocusWindow(entry.hostLabel);
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
    if (e.key === "Escape") {
      e.preventDefault();
      onDismiss(null);
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      const entry = filtered[index];
      if (entry) selectEntry(entry);
      return;
    }
    const nav = handleListNavKey(e, index, filtered.length);
    if (nav !== null) {
      e.preventDefault();
      index = nav;
      return;
    }
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

  function windowIconProvider(entry: WindowSwitchEntry) {
    return { id: entry.providerId ?? "", url: entry.providerUrl ?? undefined };
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<CommandPalette
  open={visible}
  onClose={() => onDismiss(null)}
  bind:query
  bind:inputRef
  placeholder="Search providers, windows, and actions..."
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
        bind:this={itemEls[i]}
        type="button"
        role="option"
        aria-selected={i === index}
        class="palette-item"
        class:highlight={i === index}
        onmouseenter={() => (index = i)}
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
        {:else if entry.kind === "window"}
          {@const iconSvg = providerIconSvg(windowIconProvider(entry))}
          <span
            class="palette-item-icon window-icon"
            class:muted={entry.isCurrent}
            aria-hidden="true"
          >
            {#if iconSvg}
              <span class="window-icon-base">{@html iconSvg}</span>
            {:else}
              <span class="window-icon-base placeholder"></span>
            {/if}
            <span class="window-icon-overlay">
              <AppWindow size={9} strokeWidth={2.4} />
            </span>
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

  .window-icon {
    position: relative;
  }

  .window-icon-base {
    width: 100%;
    height: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  :global(.window-icon-base svg),
  :global(.window-icon-base img) {
    width: 100%;
    height: 100%;
    display: block;
  }

  .window-icon-base.placeholder {
    background: var(--surface-overlay-faint);
    border-radius: var(--radius-xs);
  }

  .window-icon-overlay {
    position: absolute;
    right: -3px;
    bottom: -3px;
    width: 11px;
    height: 11px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--surface-floating-modal);
    border: 1px solid var(--border-default);
    border-radius: 999px;
    color: var(--text-primary);
  }

  :global(.palette-item.highlight .window-icon-overlay) {
    background: var(--surface-overlay);
  }
</style>
