<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { initTheme } from "$lib/stores/theme.svelte";
  import SettingsSidebar, {
    type SettingsSection,
  } from "$lib/components/features/settings/SettingsSidebar.svelte";
  import SettingsContent from "$lib/components/features/settings/SettingsContent.svelte";

  const store = getSettingsStore();

  let activeSection = $state<SettingsSection>("models");

  onMount(async () => {
    await initTheme();
    const initial = (window as unknown as { __settingsInitialSection?: string })
      .__settingsInitialSection;
    if (initial === "models") activeSection = "models";
    await store.init();
  });

  onDestroy(() => store.destroy());
</script>

<div class="dialog-shell">
  <SettingsSidebar bind:active={activeSection} />
  <SettingsContent {activeSection} />
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
</style>
