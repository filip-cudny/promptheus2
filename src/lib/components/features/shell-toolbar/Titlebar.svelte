<script lang="ts">
  import WindowControls from "$lib/components/shared/widgets/WindowControls.svelte";
  import type { TabDto } from "$lib/services/tabs";
  import type { WebviewProvider } from "$lib/services/aiWebview";
  import TabStrip from "./TabStrip.svelte";

  let {
    isMac,
    tabs,
    activeId,
    providers,
    isMaximized = false,
    onActivate,
    onClose,
    onOpenPlus,
    onOpenOverflow,
    onMinimize,
    onToggleMaximize,
    onWindowClose,
  }: {
    isMac: boolean;
    tabs: TabDto[];
    activeId: string | null;
    providers: WebviewProvider[];
    isMaximized?: boolean;
    onActivate: (id: string) => void;
    onClose: (id: string) => void;
    onOpenPlus: (anchor: HTMLElement) => void;
    onOpenOverflow: (anchor: HTMLElement, hidden: TabDto[]) => void;
    onMinimize: () => void;
    onToggleMaximize: () => void;
    onWindowClose: () => void;
  } = $props();
</script>

<div class="titlebar" class:mac={isMac} data-tauri-drag-region>
  <TabStrip {tabs} {activeId} {providers} {onActivate} {onClose} {onOpenPlus} {onOpenOverflow} />

  {#if !isMac}
    <WindowControls {isMaximized} {onMinimize} {onToggleMaximize} onClose={onWindowClose} />
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
</style>
