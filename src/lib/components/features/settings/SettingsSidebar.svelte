<script lang="ts" module>
  export type SettingsSection =
    | "general"
    | "notifications"
    | "history"
    | "sync"
    | "environment"
    | "models"
    | "prompt_base"
    | "skills"
    | "mcp_servers"
    | "chat"
    | "context_menu"
    | "speech";

  export interface SidebarItem {
    id: SettingsSection;
    label: string;
  }

  export interface SidebarGroup {
    label: string;
    items: SidebarItem[];
  }

  export const SIDEBAR_GROUPS: SidebarGroup[] = [
    {
      label: "App",
      items: [
        { id: "general", label: "General" },
        { id: "notifications", label: "Notifications" },
        { id: "history", label: "History" },
        { id: "sync", label: "Sync" },
        { id: "environment", label: "Environment" },
      ],
    },
    {
      label: "AI",
      items: [
        { id: "models", label: "Models" },
        { id: "prompt_base", label: "Prompt Base" },
        { id: "skills", label: "Skills" },
        { id: "mcp_servers", label: "MCP Servers" },
      ],
    },
    {
      label: "Features",
      items: [
        { id: "chat", label: "Chat" },
        { id: "context_menu", label: "Context menu" },
        { id: "speech", label: "Speech" },
      ],
    },
  ];
</script>

<script lang="ts">
  import Sidebar from "$lib/components/shared/ui/Sidebar.svelte";
  import { getSettingsNavigation } from "$lib/stores/settingsNavigation.svelte";
  import { Sparkles } from "lucide-svelte";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";
  import SettingsSearch from "./SettingsSearch.svelte";

  const agent = getSettingsAgent();

  const navigation = getSettingsNavigation();

  function sectionLabel(section: SettingsSection): string {
    return (
      SIDEBAR_GROUPS.flatMap((g) => g.items).find((i) => i.id === section)?.label ?? section
    );
  }
</script>

<Sidebar>
  <div class="sidebar-inner">
    <h1 class="sidebar-title">Settings</h1>
    <SettingsSearch {sectionLabel} />

    <button
      class="ask-agent"
      class:active={agent.panelOpen}
      onclick={() => agent.setPanelOpen(!agent.panelOpen)}
    >
      <Sparkles size={14} />
      Ask agent
    </button>

    {#each SIDEBAR_GROUPS as group (group.label)}
      <div class="group-label">{group.label}</div>
      <nav>
        {#each group.items as item (item.id)}
          <button
            class="sidebar-item"
            class:active={navigation.activeSection === item.id}
            onclick={() => (navigation.activeSection = item.id)}
          >
            {item.label}
          </button>
        {/each}
      </nav>
    {/each}
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

  .sidebar-item:hover:not(.active) {
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
  }

  .sidebar-item.active {
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
    border-left-color: var(--accent);
  }

  .ask-agent {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    margin: var(--space-2) var(--space-0);
    padding: 6px var(--space-8);
    background: transparent;
    border: none;
    border-left: 2px solid transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-base);
    text-align: left;
    cursor: pointer;
  }

  .ask-agent :global(svg) {
    color: var(--text-muted);
  }

  .ask-agent:hover {
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
  }

  .ask-agent.active {
    background: var(--surface-overlay-strong);
    color: var(--text-primary);
  }

  .ask-agent.active :global(svg) {
    color: var(--text-primary);
  }

  .group-label {
    margin: var(--space-4) var(--space-8) var(--space-2);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: var(--tracking-label);
    color: var(--text-faint);
  }
</style>
