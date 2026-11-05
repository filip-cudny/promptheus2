<script lang="ts">
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import type { SettingsSection } from "./SettingsSidebar.svelte";
  import SectionModels from "./SectionModels.svelte";
  import SectionGeneral from "./SectionGeneral.svelte";
  import SectionPromptBase from "./SectionPromptBase.svelte";
  import SectionChat from "./SectionChat.svelte";
  import SectionContextMenu from "./SectionContextMenu.svelte";
  import SectionSkills from "./SectionSkills.svelte";
  import SectionNotifications from "./SectionNotifications.svelte";
  import SectionHistory from "./SectionHistory.svelte";
  import SectionSpeech from "./SectionSpeech.svelte";
  import SectionSync from "./SectionSync.svelte";
  import SectionMcpServers from "./SectionMcpServers.svelte";

  let { activeSection }: { activeSection: SettingsSection } = $props();

  const store = getSettingsStore();
</script>

<main class="content">
  {#if !store.settings}
    {#if store.error}
      <div class="error">Failed to load settings: {store.error}</div>
    {:else}
      <div class="loading">Loading…</div>
    {/if}
  {:else if activeSection === "models"}
    <SectionModels />
  {:else if activeSection === "prompt_base"}
    <SectionPromptBase />
  {:else if activeSection === "chat"}
    <SectionChat />
  {:else if activeSection === "context_menu"}
    <SectionContextMenu />
  {:else if activeSection === "notifications"}
    <SectionNotifications />
  {:else if activeSection === "general"}
    <SectionGeneral />
  {:else if activeSection === "skills"}
    <SectionSkills />
  {:else if activeSection === "mcp_servers"}
    <SectionMcpServers />
  {:else if activeSection === "history"}
    <SectionHistory />
  {:else if activeSection === "speech"}
    <SectionSpeech />
  {:else if activeSection === "sync"}
    <SectionSync />
  {/if}
</main>

<style>
  .content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .loading,
  .error {
    margin: auto;
    color: var(--text-muted);
    font-size: var(--font-size-base);
    padding: var(--space-12);
    text-align: center;
  }

  .error {
    color: var(--danger);
  }
</style>
