<script lang="ts">
  import { getTheme, setTheme } from "$lib/stores/theme.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { updateSetting } from "$lib/services/settings";
  import { error as logError } from "@tauri-apps/plugin-log";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import FormRow from "$lib/components/shared/ui/FormRow.svelte";
  import SelectDropdown from "$lib/components/shared/ui/SelectDropdown.svelte";
  import SettingsSection from "./SettingsSection.svelte";
  import type { AppearanceSettings, Skin } from "$lib/types";

  const SKIN_OPTIONS: ReadonlyArray<{ value: Skin; label: string }> = [
    { value: "color", label: "Color" },
    { value: "mono", label: "Mono" },
  ];

  const store = getSettingsStore();
  const appearance = $derived(store.settings?.appearance ?? null);

  async function save(patch: Partial<AppearanceSettings>) {
    if (!appearance) return;
    try {
      await updateSetting("appearance", { ...appearance, ...patch });
    } catch (e) {
      logError(`update_setting appearance failed: ${e}`);
    }
  }
</script>

<SettingsSection title="Appearance">
  {#snippet body()}
    <div class="group">
      <FormRow label="Theme">
        <Button onclick={() => setTheme(getTheme() === "dark" ? "light" : "dark")}>
          {getTheme() === "dark" ? "Light mode" : "Dark mode"}
        </Button>
      </FormRow>
      {#if appearance}
        <FormRow label="Logo">
          <SelectDropdown
            value={appearance.logo}
            options={SKIN_OPTIONS}
            ariaLabel="Logo"
            onchange={(logo) => save({ logo })}
          />
        </FormRow>
        <FormRow label="Icons">
          <SelectDropdown
            value={appearance.icons}
            options={SKIN_OPTIONS}
            ariaLabel="Icons"
            onchange={(icons) => save({ icons })}
          />
        </FormRow>
        <p class="muted">Applies to notification icons and the check in the recording widget.</p>
      {/if}
    </div>
  {/snippet}
</SettingsSection>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-width: 560px;
  }

  .muted {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }
</style>
