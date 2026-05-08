<script lang="ts">
  import { onMount, onDestroy, untrack } from "svelte";
  import SettingsSection from "./SettingsSection.svelte";
  import PromptEditor from "./PromptEditor.svelte";
  import PromptTabs from "./PromptTabs.svelte";
  import TextInput from "$lib/components/shared/ui/TextInput.svelte";
  import SaveStatusIndicator from "$lib/components/shared/widgets/SaveStatusIndicator.svelte";
  import { useSaveTracker } from "$lib/stores/saveTracker.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { updateSetting } from "$lib/services/settings";
  import type { PromptKind } from "$lib/services/prompts";

  type TabId = Extract<PromptKind, "system" | "about_you" | "environment" | "input_format">;

  const PREFERRED_NAME_MAX = 60;

  const TABS: ReadonlyArray<{ id: TabId; label: string; description: string }> = [
    {
      id: "system",
      label: "System",
      description: "Top-level instructions sent as the assistant's system role.",
    },
    {
      id: "about_you",
      label: "About you",
      description:
        "Long-form context about the user — wrapped in <user_context> in every conversation. This is about you, not about the AI.",
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

  const settingsStore = getSettingsStore();

  let preferredName = $state("");
  let savedPreferredName = $state("");
  let initialized = false;

  const tracker = useSaveTracker({
    debounceMs: settingsStore.settings?.autosave_debounce_ms ?? 1000,
  });

  $effect(() => {
    const remote = settingsStore.settings?.preferred_name ?? "";
    if (!initialized) {
      untrack(() => {
        preferredName = remote;
        savedPreferredName = remote;
      });
      initialized = true;
      return;
    }
    if (remote !== savedPreferredName && !tracker.dirty && !tracker.saving) {
      untrack(() => {
        preferredName = remote;
        savedPreferredName = remote;
      });
    }
  });

  $effect(() => {
    if (!initialized) return;
    const _track = preferredName;
    if (preferredName === savedPreferredName) {
      tracker.cancel();
      return;
    }
    tracker.scheduleSave(persistPreferredName);
  });

  async function persistPreferredName() {
    const trimmed = preferredName.trim().slice(0, PREFERRED_NAME_MAX);
    if (trimmed !== preferredName) {
      preferredName = trimmed;
    }
    if (trimmed === savedPreferredName) return;
    await updateSetting("preferred_name", trimmed);
    savedPreferredName = trimmed;
  }

  onMount(() => {
    tracker.attachKeyboard(window);
    tracker.attachBeforeUnload(window);
  });

  onDestroy(() => {
    tracker.destroy();
  });

  const remaining = $derived(PREFERRED_NAME_MAX - preferredName.length);
  const overLimit = $derived(remaining < 0);
</script>

<SettingsSection
  title="Prompt Base"
  hint="Templates that build the system message of every chat conversation."
>
  {#snippet body()}
    <div class="prompt-base-body">
      <section class="identity-card">
        <header class="identity-head">
          <div class="identity-text">
            <h3>Preferred name</h3>
            <p class="helper">The name the AI will use to address you. Leave empty to skip.</p>
          </div>
          <SaveStatusIndicator {tracker} />
        </header>
        <div class="identity-input">
          <TextInput
            bind:value={preferredName}
            placeholder=""
            error={overLimit}
            maxlength={PREFERRED_NAME_MAX}
            autocomplete="off"
            spellcheck="false"
          />
          <span class="counter" class:over={overLimit}>{remaining}</span>
        </div>
      </section>

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

  .identity-card {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-5) var(--space-6);
    background: var(--surface-elevated);
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-md);
    width: 100%;
    max-width: 760px;
  }

  .identity-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .identity-text {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }

  .identity-text h3 {
    margin: 0;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--text-muted);
  }

  .helper {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .identity-input {
    position: relative;
    display: flex;
    align-items: center;
  }

  .counter {
    position: absolute;
    right: var(--space-3);
    font-size: var(--font-size-xs);
    font-variant-numeric: tabular-nums;
    color: var(--text-faint);
    pointer-events: none;
  }

  .counter.over {
    color: var(--danger);
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
