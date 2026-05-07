<script lang="ts">
  import SettingsSection from "./SettingsSection.svelte";
  import PromptEditor from "./PromptEditor.svelte";
  import PromptTabs from "./PromptTabs.svelte";
  import type { PromptKind } from "$lib/services/prompts";

  type TabId = Extract<PromptKind, "system" | "about_me" | "environment" | "input_format">;

  const TABS: ReadonlyArray<{ id: TabId; label: string; description: string }> = [
    {
      id: "system",
      label: "System",
      description: "Top-level instructions sent as the assistant's system role.",
    },
    {
      id: "about_me",
      label: "About me",
      description: "Long-form context about you. Appended to every conversation's system message.",
    },
    {
      id: "environment",
      label: "Environment",
      description:
        "Per-conversation snapshot built when chat starts. Use placeholders for dynamic values.",
    },
    {
      id: "input_format",
      label: "Input format",
      description:
        "Tags and conventions the model should expect in user messages (context, pasted text, images, skills).",
    },
  ];

  let active = $state<TabId>("system");
  const activeTab = $derived(TABS.find((t) => t.id === active) ?? TABS[0]);
</script>

<SettingsSection
  title="Prompt Base"
  hint="Templates that build the system message of every chat conversation."
>
  {#snippet body()}
    <div class="prompt-base-body">
      <PromptTabs tabs={TABS.map((t) => ({ id: t.id, label: t.label }))} bind:active />
      <div class="editor-wrap" class:wide={activeTab.id === "environment"}>
        {#key activeTab.id}
          <PromptEditor kind={activeTab.id} description={activeTab.description} />
        {/key}
      </div>
    </div>
  {/snippet}
</SettingsSection>

<style>
  .prompt-base-body {
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

  .editor-wrap.wide {
    max-width: 960px;
  }
</style>
