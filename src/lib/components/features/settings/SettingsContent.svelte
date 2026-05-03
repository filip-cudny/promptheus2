<script lang="ts">
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import type { SettingsSection } from "./SettingsSidebar.svelte";
  import SectionModels from "./SectionModels.svelte";
  import SectionAppearance from "./SectionAppearance.svelte";

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
  {:else if activeSection === "appearance"}
    <SectionAppearance />
  {:else}
    <div class="placeholder">This section is not yet implemented.</div>
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
