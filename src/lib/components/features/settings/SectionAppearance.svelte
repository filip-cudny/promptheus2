<script lang="ts">
  import { getTheme, setTheme } from "$lib/stores/theme.svelte";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import FormRow from "$lib/components/shared/ui/FormRow.svelte";
  import SettingsSection from "./SettingsSection.svelte";
</script>

<SettingsSection title="Appearance">
  {#snippet body()}
    <FormRow label="Theme">
      <Button onclick={() => setTheme(getTheme() === "dark" ? "light" : "dark")}>
        {getTheme() === "dark" ? "Light mode" : "Dark mode"}
      </Button>
    </FormRow>
  {/snippet}
</SettingsSection>
