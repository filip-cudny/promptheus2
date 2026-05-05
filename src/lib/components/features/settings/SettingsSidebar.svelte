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

  function select(item: SidebarItem) {
    if (!item.enabled) return;
    active = item.id;
  }
</script>

<Sidebar>
  <div class="sidebar-inner">
    <h1 class="sidebar-title">Settings</h1>
    <nav>
      {#each SIDEBAR_ITEMS as item (item.id)}
        <button
          class="sidebar-item"
          class:active={active === item.id}
          class:disabled={!item.enabled}
          title={item.enabled ? undefined : "Coming soon"}
          disabled={!item.enabled}
          onclick={() => select(item)}
        >
          {item.label}
        </button>
      {/each}
    </nav>
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
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    letter-spacing: 0.6px;
    text-transform: uppercase;
    color: var(--text-disabled);
  }

  nav {
    display: flex;
    flex-direction: column;
  }

  .sidebar-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 7px var(--space-8);
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-base);
    cursor: pointer;
  }

  .sidebar-item:hover:not(.disabled):not(.active) {
    background: var(--surface-overlay-faint);
  }

  .sidebar-item.active {
    background: var(--accent-bg-soft);
    color: var(--accent);
  }

  .sidebar-item.disabled {
    color: var(--text-faint);
    cursor: default;
  }
</style>
