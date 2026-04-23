<script lang="ts" module>
  export type SettingsSection =
    | "general"
    | "models"
    | "prompt_base"
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
    { id: "prompt_base", label: "Prompt Base", enabled: false },
    { id: "surfaces", label: "Surfaces", enabled: false },
    { id: "notifications", label: "Notifications", enabled: false },
    { id: "appearance", label: "Appearance", enabled: false },
    { id: "shortcuts", label: "Shortcuts", enabled: false },
    { id: "skills", label: "Skills", enabled: false },
    { id: "mcp_servers", label: "MCP Servers", enabled: false },
    { id: "advanced", label: "Advanced", enabled: false },
  ];
</script>

<script lang="ts">
  let { active = $bindable<SettingsSection>("models") }: { active: SettingsSection } = $props();

  function select(item: SidebarItem) {
    if (!item.enabled) return;
    active = item.id;
  }
</script>

<aside class="sidebar">
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
</aside>

<style>
  .sidebar {
    width: 220px;
    flex-shrink: 0;
    background: #1a1a1a;
    border-right: 1px solid rgba(255, 255, 255, 0.06);
    padding: 16px 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
  }

  .sidebar-title {
    margin: 0 16px 12px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.6px;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.35);
  }

  nav {
    display: flex;
    flex-direction: column;
  }

  .sidebar-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 7px 16px;
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.78);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }

  .sidebar-item:hover:not(.disabled):not(.active) {
    background: rgba(255, 255, 255, 0.04);
  }

  .sidebar-item.active {
    background: rgba(91, 141, 217, 0.15);
    color: #8db3ee;
  }

  .sidebar-item.disabled {
    color: rgba(255, 255, 255, 0.25);
    cursor: default;
  }
</style>
