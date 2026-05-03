<script lang="ts">
  import { RefreshCw } from "lucide-svelte";
  import CommandPalette from "$lib/components/shared/ui/CommandPalette.svelte";
  import KbdHint from "$lib/components/shared/ui/KbdHint.svelte";
  import { providerIconSvg } from "$lib/icons/providerIcons";
  import { PROMPTHEUS_PROVIDER_ID } from "$lib/services/shellToolbar";
  import { handleListNavKey } from "$lib/utils/listNavigation";
  import { SHORTCUTS, matches } from "$lib/shortcuts";

  type ProviderEntry = { kind: "provider"; id: string; name: string; url?: string };
  type ActionEntry = { kind: "action"; id: string; name: string };
  type PaletteEntry = ProviderEntry | ActionEntry;

  const ACTION_RELOAD_ID = "action:reload-active";

  let {
    visible,
    activeId,
    webviewProviders,
    inputRef = $bindable(undefined),
    onDismiss,
    onReloadActive,
  }: {
    visible: boolean;
    activeId: string;
    webviewProviders: { id: string; name: string; url?: string | null }[];
    inputRef?: HTMLInputElement | undefined;
    onDismiss: (selectedId: string | null) => void;
    onReloadActive: () => void;
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

  let actions = $derived<ActionEntry[]>([
    { kind: "action", id: ACTION_RELOAD_ID, name: `Reload ${activeName}` },
  ]);

  let entries = $derived<PaletteEntry[]>([...providers, ...actions]);

  let filtered = $derived.by<PaletteEntry[]>(() => {
    const q = query.trim().toLowerCase();
    if (!q) return entries;
    return entries.filter((e) => e.name.toLowerCase().includes(q));
  });

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
</script>

<svelte:window onkeydown={handleKeydown} />

<CommandPalette
  open={visible}
  onClose={() => onDismiss(null)}
  bind:query
  bind:inputRef
  placeholder="Search providers and actions..."
  variant="window"
  bodyMaxHeight="320px"
>
  {#snippet body()}
    {#each filtered as entry, i (entry.id)}
      {#if entry.kind === "action" && i > 0 && filtered[i - 1].kind === "provider"}
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
        <span class="palette-item-icon" class:muted={entry.kind === "action"} aria-hidden="true">
          {#if entry.kind === "provider"}
            {@const iconSvg = providerIconSvg(entry)}
            {#if iconSvg}
              {@html iconSvg}
            {/if}
          {:else}
            <RefreshCw size={14} />
          {/if}
        </span>
        <span class="palette-item-name">{entry.name}</span>
        {#if entry.kind === "provider" && entry.id === activeId}
          <span class="badge">active</span>
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

  .badge {
    font-size: var(--font-size-xs);
    text-transform: uppercase;
    color: var(--text-muted);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm);
    padding: 1px 5px;
  }
</style>
