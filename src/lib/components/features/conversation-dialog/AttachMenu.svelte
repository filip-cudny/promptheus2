<script lang="ts">
  import { Plus, FileText } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import type { ComponentType, SvelteComponent } from "svelte";
  import type { IconProps } from "lucide-svelte";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";
  import MenuList from "$lib/components/shared/ui/MenuList.svelte";
  import MenuItem from "$lib/components/shared/ui/MenuItem.svelte";

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
  <span class="attach-btn-wrap">
    <ActionIconButton
      icon={Plus}
      size={ICON_SIZE.md}
      onclick={() => (menuOpen = !menuOpen)}
      title="Add attachment"
    />
  </span>

  {#if menuOpen}
    <div class="menu-dropdown">
      <MenuList role="menu">
        <MenuItem
          label="Context"
          disabled={contextDisabled}
          onclick={handleContextClick}
        >
          {#snippet icon()}
            <FileText size={ICON_SIZE.md} />
          {/snippet}
        </MenuItem>
        {#if availableTools.length > 0}
          <div class="menu-list-separator"></div>
          {#each availableTools as tool (tool.id)}
            <MenuItem
              label={tool.label}
              active={tool.active}
              onclick={() => { onToggleTool?.(tool.id, !tool.active); menuOpen = false; }}
            >
              {#snippet icon()}
                {#if tool.icon}
                  {@const Icon = tool.icon}
                  <Icon size={ICON_SIZE.md} />
                {/if}
              {/snippet}
            </MenuItem>
          {/each}
        {/if}
      </MenuList>
    </div>
  {/if}
</div>

<style>
  .attach-menu {
    position: relative;
    flex-shrink: 0;
    align-self: flex-end;
  }

  .attach-btn-wrap {
    display: flex;
  }

  .attach-btn-wrap :global(.action-icon-btn) {
    width: 24px;
    height: 24px;
    padding: var(--space-0);
    border-radius: var(--radius-lg);
    color: var(--text-muted);
  }

  .menu-dropdown {
    position: absolute;
    bottom: calc(100% + 4px);
    left: 0;
    min-width: 160px;
    background: var(--surface-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-md);
    padding: var(--space-2) var(--space-0);
    z-index: var(--z-dropdown);
  }
</style>
