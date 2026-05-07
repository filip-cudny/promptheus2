<script lang="ts">
  import SettingsSection from "./SettingsSection.svelte";
  import PromptEditor from "./PromptEditor.svelte";
  import PromptTabs from "./PromptTabs.svelte";
  import type { PromptKind } from "$lib/services/prompts";

  type TabId = Extract<PromptKind, "title_generation" | "speech_to_text">;

  const TABS: ReadonlyArray<{ id: TabId; label: string; description: string }> = [
    {
      id: "title_generation",
      label: "Title generation",
      description: "Used to auto-name conversations after the first user message.",
    },
    {
      id: "speech_to_text",
      label: "Speech-to-text",
      description: "Optional bias prompt for the speech-to-text model. Leave empty to disable.",
    },
  ];

  let active = $state<TabId>("title_generation");
  const activeTab = $derived(TABS.find((t) => t.id === active) ?? TABS[0]);
</script>

<SettingsSection
  title="Surface prompts"
  hint="Per-feature prompts used outside the main chat surface."
>
  {#snippet body()}
    <div class="surface-prompts-body">
      <PromptTabs tabs={TABS.map((t) => ({ id: t.id, label: t.label }))} bind:active />
      <div class="editor-wrap">
        {#key activeTab.id}
          <PromptEditor kind={activeTab.id} description={activeTab.description} />
        {/key}
      </div>
    </div>
  {/snippet}
</SettingsSection>

<style>
  .surface-prompts-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    flex: 1;
    min-height: 0;
  }

  .editor-wrap {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    width: 100%;
    max-width: 760px;
  }
</style>
