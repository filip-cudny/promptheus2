<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { initTheme } from "$lib/stores/theme.svelte";
  import SettingsSidebar, {
    type SettingsSection,
  } from "$lib/components/features/settings/SettingsSidebar.svelte";
  import SectionModels from "$lib/components/features/settings/SectionModels.svelte";
  import SectionAppearance from "$lib/components/features/settings/SectionAppearance.svelte";

  const store = getSettingsStore();

  let activeSection = $state<SettingsSection>("models");

  onMount(async () => {
    await initTheme();
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
    {:else if activeSection === "appearance"}
      <SectionAppearance />
    {:else}
      <div class="placeholder">This section is not yet implemented.</div>
    {/if}
  </main>
</div>

<style>
  .dialog-shell {
    display: flex;
    height: 100vh;
    background: var(--surface-base);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--font-size-base);
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
    color: var(--text-muted);
    font-size: var(--font-size-base);
    padding: var(--space-12);
    text-align: center;
  }

  .error {
    color: var(--danger);
  }
</style>
