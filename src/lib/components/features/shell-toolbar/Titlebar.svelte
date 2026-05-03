<script lang="ts">
  import { SquareArrowOutUpRight } from "lucide-svelte";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import WindowControls from "$lib/components/shared/widgets/WindowControls.svelte";
  import ProviderSwitcherTrigger from "./ProviderSwitcherTrigger.svelte";

  type Provider = { id: string; name: string; url?: string };

  let {
    isMac,
    shortcutHint,
    activeProvider,
    providerDropdownOpen = false,
    triggerEl = $bindable(null),
    isMaximized = false,
    onToggleProviderDropdown,
    onOpenInNewWindow,
    onMinimize,
    onToggleMaximize,
    onClose,
  }: {
    isMac: boolean;
    shortcutHint: string;
    activeProvider: Provider | undefined;
    providerDropdownOpen?: boolean;
    triggerEl?: HTMLButtonElement | null;
    isMaximized?: boolean;
    onToggleProviderDropdown: (e: MouseEvent) => void;
    onOpenInNewWindow: () => void;
    onMinimize: () => void;
    onToggleMaximize: () => void;
    onClose: () => void;
  } = $props();
</script>

<div class="titlebar" class:mac={isMac} data-tauri-drag-region>
  <span class="hint" title="Open command palette">{shortcutHint}</span>

  <div class="switcher">
    <ProviderSwitcherTrigger
      {activeProvider}
      expanded={providerDropdownOpen}
      bind:triggerEl
      onToggle={onToggleProviderDropdown}
    />
  </div>

  <Button variant="chrome" title="Open in new window" onclick={onOpenInNewWindow}>
    <SquareArrowOutUpRight size={14} />
  </Button>

  <div class="drag-fill" data-tauri-drag-region></div>

  {#if !isMac}
    <WindowControls {isMaximized} {onMinimize} {onToggleMaximize} {onClose} />
  {/if}
</div>

<style>
  .titlebar {
    height: 40px;
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-0) var(--space-2) var(--space-0) var(--space-4);
    background: var(--surface-base);
    border-bottom: 1px solid var(--border-faint);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--font-size-md);
    user-select: none;
    -webkit-user-select: none;
    box-sizing: border-box;
  }

  .titlebar.mac {
    padding-left: 80px;
  }

  .switcher {
    position: relative;
    display: inline-flex;
  }

  .drag-fill {
    flex: 1;
    align-self: stretch;
  }

  .hint {
    color: var(--text-disabled);
    font-size: var(--font-size-sm);
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-default);
  }
</style>
