<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import SettingsSidebar, {
    type SettingsSection,
  } from "$lib/components/settings/SettingsSidebar.svelte";
  import SectionModels from "$lib/components/settings/SectionModels.svelte";

  const store = getSettingsStore();

  let activeSection = $state<SettingsSection>("models");

  onMount(async () => {
    const initial = (window as unknown as { __settingsInitialSection?: string })
      .__settingsInitialSection;
    if (initial === "models") {
      activeSection = "models";
    }
    await store.init();
  });

  onDestroy(() => {
    store.destroy();
  });
</script>

<div class="dialog-shell">
  <SettingsSidebar bind:active={activeSection} />

  <main class="content">
    {#if !store.settings}
      {#if store.error}
        <div class="error">Failed to load settings: {store.error}</div>
      {:else}
        <div class="loading">Loading…</div>
      {/if}
    {:else if activeSection === "models"}
      <SectionModels />
    {:else}
      <div class="placeholder">This section is not yet implemented.</div>
    {/if}
  </main>
</div>

<style>
  .dialog-shell {
    display: flex;
    height: 100vh;
    background: #1e1e1e;
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 13px;
    overflow: hidden;
  }

  .content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .loading,
  .error,
  .placeholder {
    margin: auto;
    color: rgba(255, 255, 255, 0.45);
    font-size: 13px;
    padding: 24px;
    text-align: center;
  }

  .error {
    color: #d97373;
  }
</style>
