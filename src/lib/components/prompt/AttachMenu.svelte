<script lang="ts">
  import { Plus, FileText, Check } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import type { ComponentType, SvelteComponent } from "svelte";
  import type { IconProps } from "lucide-svelte";

  type LucideIcon = ComponentType<SvelteComponent<IconProps>>;

  let {
    onSelectContext,
    contextDisabled = false,
    availableTools = [],
    onToggleTool,
  }: {
    onSelectContext: () => void;
    contextDisabled?: boolean;
    availableTools?: { id: string; label: string; icon?: LucideIcon; active: boolean }[];
    onToggleTool?: (toolId: string, enabled: boolean) => void;
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
      {#if availableTools.length > 0}
        <div class="menu-separator"></div>
        {#each availableTools as tool (tool.id)}
          <button
            class="menu-item"
            onclick={() => { onToggleTool?.(tool.id, !tool.active); menuOpen = false; }}
          >
            <span class="tool-check" class:tool-check-active={tool.active}>
              {#if tool.active}<Check size={12} />{/if}
            </span>
            {#if tool.icon}
              {@const Icon = tool.icon}
              <Icon size={ICON_SIZE.md} />
            {/if}
            <span>{tool.label}</span>
          </button>
        {/each}
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
    width: 24px;
    height: 24px;
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


  .tool-check {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    flex-shrink: 0;
    color: transparent;
  }

  .tool-check-active {
    color: #5b8dd9;
  }

</style>
