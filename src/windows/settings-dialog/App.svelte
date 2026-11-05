<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { syncTheme } from "$lib/stores/theme.svelte";
  import SettingsSidebar from "$lib/components/features/settings/SettingsSidebar.svelte";
  import { getSettingsNavigation } from "$lib/stores/settingsNavigation.svelte";
  import SettingsContent from "$lib/components/features/settings/SettingsContent.svelte";
  import MissingEnvList from "$lib/components/features/settings/MissingEnvList.svelte";
  import SettingsErrorToast from "$lib/components/features/settings/SettingsErrorToast.svelte";

  const store = getSettingsStore();

  const navigation = getSettingsNavigation();

  onMount(async () => {
    const initial = (window as unknown as { __settingsInitialSection?: string })
      .__settingsInitialSection;
    if (initial === "models") navigation.activeSection = "models";
    await store.init();
  });

  $effect(() => {
    if (store.settings) syncTheme(store.settings.theme);
  });

  onDestroy(() => store.destroy());
</script>

<div class="dialog-shell">
  <MissingEnvList />
  <div class="dialog-body">
    <SettingsSidebar />
    <SettingsContent />
  </div>
  <SettingsErrorToast />
</div>

<style>
  .dialog-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--surface-base);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--font-size-base);
    overflow: hidden;
  }

  .dialog-body {
    display: flex;
    flex: 1;
    min-height: 0;
  }
</style>
