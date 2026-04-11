<script lang="ts">
  import { Plus, FileText } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    onSelectContext,
    contextDisabled = false,
    showWebSearchSwitch = false,
    webSearchProvider = "builtin" as "builtin" | "mcp",
    onWebSearchProviderChange,
  }: {
    onSelectContext: () => void;
    contextDisabled?: boolean;
    showWebSearchSwitch?: boolean;
    webSearchProvider?: "builtin" | "mcp";
    onWebSearchProviderChange?: (provider: "builtin" | "mcp") => void;
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
      {#if showWebSearchSwitch}
        <div class="menu-separator"></div>
        <div class="menu-label">Web Search</div>
        <div class="provider-switch">
          <button
            class="provider-option"
            class:selected={webSearchProvider === "builtin"}
            onclick={() => onWebSearchProviderChange?.("builtin")}
          >Built-in</button>
          <button
            class="provider-option"
            class:selected={webSearchProvider === "mcp"}
            onclick={() => onWebSearchProviderChange?.("mcp")}
          >MCP</button>
        </div>
      {/if}
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

  .menu-separator {
    height: 1px;
    background: rgba(255, 255, 255, 0.1);
    margin: 4px 0;
  }

  .menu-label {
    padding: 4px 12px 2px;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.4);
    user-select: none;
  }

  .provider-switch {
    display: flex;
    gap: 2px;
    margin: 2px 8px 4px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 4px;
    padding: 2px;
  }

  .provider-option {
    flex: 1;
    padding: 4px 8px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    text-align: center;
  }

  .provider-option:hover:not(.selected) {
    color: rgba(255, 255, 255, 0.7);
  }

  .provider-option.selected {
    background: rgba(255, 255, 255, 0.12);
    color: #e0e0e0;
  }

</style>
