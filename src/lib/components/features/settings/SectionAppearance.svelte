<script lang="ts">
  import { getTheme, setTheme } from "$lib/stores/theme.svelte";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import Section from "$lib/components/shared/ui/Section.svelte";
  import FormRow from "$lib/components/shared/ui/FormRow.svelte";
</script>

<div class="section-appearance">
  <Section title="Appearance">
    <FormRow label="Theme">
      <Button onclick={() => setTheme(getTheme() === "dark" ? "light" : "dark")}>
        {getTheme() === "dark" ? "Light mode" : "Dark mode"}
      </Button>
    </FormRow>
  </Section>
</div>

<style>
  .section-appearance {
    padding: var(--space-8);
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
  }
</style>
