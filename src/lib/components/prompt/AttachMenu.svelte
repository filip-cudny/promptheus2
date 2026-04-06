<script lang="ts">
  import { Plus, FileText, Globe } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    onSelectContext,
    onToggleWebSearch,
    contextDisabled = false,
    webSearchEnabled = false,
    webSearchAvailable = false,
  }: {
    onSelectContext: () => void;
    onToggleWebSearch: () => void;
    contextDisabled?: boolean;
    webSearchEnabled?: boolean;
    webSearchAvailable?: boolean;
  } = $props();

  let menuOpen = $state(false);
  let containerEl: HTMLDivElement | undefined = $state();

  function handleWindowPointerDown(e: PointerEvent) {
    if (menuOpen && containerEl && !containerEl.contains(e.target as Node)) {
      menuOpen = false;
    }
  }

  function handleContextClick() {
    if (contextDisabled) return;
    menuOpen = false;
    onSelectContext();
  }

  function handleWebSearchClick() {
    if (!webSearchAvailable) return;
    menuOpen = false;
    onToggleWebSearch();
  }
</script>

<svelte:window onpointerdown={handleWindowPointerDown} />

<div class="attach-menu" bind:this={containerEl}>
  <button
    class="attach-btn"
    onclick={() => (menuOpen = !menuOpen)}
    title="Add attachment"
  >
    <Plus size={ICON_SIZE.md} />
  </button>

  {#if menuOpen}
    <div class="menu-dropdown">
      <button
        class="menu-item"
        class:disabled={contextDisabled}
        onclick={handleContextClick}
      >
        <FileText size={ICON_SIZE.md} />
        <span>Context</span>
      </button>
      <button
        class="menu-item"
        class:disabled={!webSearchAvailable}
        class:active={webSearchEnabled}
        onclick={handleWebSearchClick}
      >
        <Globe size={ICON_SIZE.md} />
        <span>Web Search</span>
        {#if webSearchEnabled}
          <span class="check">✓</span>
        {/if}
      </button>
    </div>
  {/if}
</div>

<style>
  .attach-menu {
    position: relative;
    flex-shrink: 0;
    align-self: flex-end;
  }

  .attach-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: #aaa;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .attach-btn:hover {
    color: #e0e0e0;
    background: rgba(255, 255, 255, 0.08);
  }

  .menu-dropdown {
    position: absolute;
    bottom: calc(100% + 4px);
    left: 0;
    min-width: 160px;
    background: #2a2a2a;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    box-shadow: 0 -4px 12px rgba(0, 0, 0, 0.3);
    padding: 4px 0;
    z-index: 100;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    background: none;
    border: none;
    color: #e0e0e0;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    text-align: left;
  }

  .menu-item:hover:not(.disabled) {
    background: rgba(255, 255, 255, 0.08);
  }

  .menu-item.disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .menu-item.active {
    color: #5b8dd9;
  }

  .check {
    margin-left: auto;
    font-size: 12px;
    color: #5b8dd9;
  }
</style>
