<script lang="ts" module>
  export type SettingsSection =
    | "general"
    | "models"
    | "prompt_base"
    | "surface_prompts"
    | "surfaces"
    | "notifications"
    | "appearance"
    | "shortcuts"
    | "skills"
    | "mcp_servers"
    | "advanced";

  export interface SidebarItem {
    id: SettingsSection;
    label: string;
    enabled: boolean;
  }

  export const SIDEBAR_ITEMS: SidebarItem[] = [
    { id: "general", label: "General", enabled: false },
    { id: "models", label: "Models", enabled: true },
    { id: "prompt_base", label: "Prompt Base", enabled: true },
    { id: "surface_prompts", label: "Surface Prompts", enabled: true },
    { id: "surfaces", label: "Surfaces", enabled: false },
    { id: "notifications", label: "Notifications", enabled: false },
    { id: "appearance", label: "Appearance", enabled: true },
    { id: "shortcuts", label: "Shortcuts", enabled: false },
    { id: "skills", label: "Skills", enabled: false },
    { id: "mcp_servers", label: "MCP Servers", enabled: false },
    { id: "advanced", label: "Advanced", enabled: false },
  ];
</script>

<script lang="ts">
  import Sidebar from "$lib/components/shared/ui/Sidebar.svelte";

  let { active = $bindable<SettingsSection>("models") }: { active: SettingsSection } = $props();

  const enabledItems = SIDEBAR_ITEMS.filter((i) => i.enabled);
  const comingSoonItems = SIDEBAR_ITEMS.filter((i) => !i.enabled);

  function select(item: SidebarItem) {
    if (!item.enabled) return;
    active = item.id;
  }
</script>

<Sidebar>
  <div class="sidebar-inner">
    <h1 class="sidebar-title">Settings</h1>

    <nav>
      {#each enabledItems as item (item.id)}
        <button
          class="sidebar-item"
          class:active={active === item.id}
          onclick={() => select(item)}
        >
          {item.label}
        </button>
      {/each}
    </nav>

    {#if comingSoonItems.length > 0}
      <div class="group-label">Coming soon</div>
      <nav class="coming-soon">
        {#each comingSoonItems as item (item.id)}
          <button
            class="sidebar-item disabled"
            title="Coming soon"
            disabled
            onclick={() => select(item)}
          >
            <span class="label">{item.label}</span>
          </button>
        {/each}
      </nav>
    {/if}
  </div>
</Sidebar>

<style>
  .sidebar-inner {
    padding: var(--space-8) var(--space-0);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    overflow-y: auto;
    height: 100%;
  }

  .sidebar-title {
    margin: var(--space-0) var(--space-8) var(--space-6);
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-semibold);
    letter-spacing: 0;
    color: var(--text-secondary);
  }

  nav {
    display: flex;
    flex-direction: column;
  }

  .sidebar-item {
    position: relative;
    display: block;
    width: 100%;
    text-align: left;
    padding: 7px var(--space-8);
    background: transparent;
    border: none;
    border-left: 2px solid transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-base);
    cursor: pointer;
    transition: background var(--motion-fast) var(--ease-default),
      color var(--motion-fast) var(--ease-default),
      border-color var(--motion-fast) var(--ease-default);
  }

  .sidebar-item:hover:not(.disabled):not(.active) {
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
  }

  .sidebar-item.active {
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
    border-left-color: var(--accent);
  }

  .sidebar-item.disabled {
    color: var(--text-faint);
    cursor: default;
    padding-block: 5px;
  }

  .group-label {
    margin: var(--space-4) var(--space-8) var(--space-2);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: var(--tracking-label);
    color: var(--text-faint);
  }

  .coming-soon .sidebar-item {
    font-size: var(--font-size-sm);
  }
</style>
