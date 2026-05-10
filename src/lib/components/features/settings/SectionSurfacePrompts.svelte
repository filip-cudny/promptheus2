<script lang="ts">
  import SettingsSection from "./SettingsSection.svelte";
  import PromptEditor from "./PromptEditor.svelte";
  import PromptTabs from "./PromptTabs.svelte";
  import SttKeytermsEditor from "./SttKeytermsEditor.svelte";
  import type { PromptKind } from "$lib/services/prompts";

  type PromptTabId = Extract<PromptKind, "title_generation" | "speech_to_text">;
  type TabId = PromptTabId | "stt_keyterms";

  const TABS: ReadonlyArray<{ id: TabId; label: string; description: string }> = [
    {
      id: "title_generation",
      label: "Title generation",
      description: "Used to auto-name conversations after the first user message.",
    },
    {
      id: "speech_to_text",
      label: "STT prompt",
      description: "Optional bias prompt for the speech-to-text model. Leave empty to disable.",
    },
    {
      id: "stt_keyterms",
      label: "STT keyterms",
      description:
        "Domain words to bias transcription — names, jargon, brands, acronyms. One term per line.",
    },
  ];

  let active = $state<TabId>("title_generation");
  const activeTab = $derived(TABS.find((t) => t.id === active) ?? TABS[0]);
</script>

<SettingsSection
  title="Surface prompts"
  hint="Per-feature prompts and biases used outside the main chat surface."
>
  {#snippet body()}
    <div class="surface-prompts-body">
      <PromptTabs tabs={TABS.map((t) => ({ id: t.id, label: t.label }))} bind:active />
      <div class="editor-wrap">
        {#key activeTab.id}
          {#if activeTab.id === "stt_keyterms"}
            <SttKeytermsEditor description={activeTab.description} />
          {:else}
            <PromptEditor
              kind={activeTab.id as PromptTabId}
              description={activeTab.description}
            />
          {/if}
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
