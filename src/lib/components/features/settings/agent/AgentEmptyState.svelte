<script lang="ts">
  import { KeyRound, MessageCircleQuestion, PencilLine, Plus, Search, Sparkles } from "lucide-svelte";
  import { error as logError } from "@tauri-apps/plugin-log";
  import { ICON_SIZE } from "$lib/constants/ui";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import SelectDropdown from "$lib/components/shared/ui/SelectDropdown.svelte";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";
  import { getSettingsNavigation } from "$lib/stores/settingsNavigation.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { updateSurfaceModel } from "$lib/services/settings";
  import { agentModelState, useModelWrites } from "$lib/utils/agentModel";

  const agent = getSettingsAgent();
  const navigation = getSettingsNavigation();
  const store = getSettingsStore();

  const lines = [
    { icon: Search, text: "Find where an option is" },
    { icon: MessageCircleQuestion, text: "Ask how something is set" },
    { icon: PencilLine, text: "Change it: you approve a card first" },
    { icon: KeyRound, text: "Secrets: shown to you masked, never to the agent" },
  ];

  const chips = [
    "Where is the context menu shortcut?",
    "Which model does Quick actions use?",
    "Where do I set the speech language?",
  ];

  let picked = $state("");
  let applying = $state(false);
  let writeError = $state<string | null>(null);

  const modelState = $derived(
    store.settings ? agentModelState(store.settings.surfaces, store.models) : "ok",
  );
  const pickOptions = $derived([
    { value: "", label: "Select a model…" },
    ...store.models
      .filter((m) => m.type === "text")
      .map((m) => ({ value: m.id, label: m.display_name || m.model })),
  ]);
  const pickedIsText = $derived(pickOptions.some((o) => o.value !== "" && o.value === picked));

  async function useModel() {
    if (!store.settings || !pickedIsText) return;
    const writes = useModelWrites(store.settings.surfaces, store.models, picked);
    applying = true;
    writeError = null;
    try {
      for (const write of writes) await updateSurfaceModel(write.surface, write.modelId);
    } catch (e) {
      writeError = String(e);
      logError(`update_surface_model ${picked} failed: ${e}`);
    } finally {
      applying = false;
    }
  }
</script>

<div class="island">
  {#if modelState === "ok"}
    <div class="title"><Sparkles size={ICON_SIZE.md} />Ask instead of searching</div>
    {@render lineList()}
    <div class="chips">
      {#each chips as chip (chip)}
        <button class="chip" onclick={() => agent.fillDraft(chip)}>{chip}</button>
      {/each}
    </div>
  {:else if modelState === "no_text_model"}
    <div class="title"><Sparkles size={ICON_SIZE.md} />The agent needs a model</div>
    <p class="msg">There is no text model yet.</p>
    <Button variant="primary" onclick={() => navigation.navigateTo("models:new")}>
      <Plus size={ICON_SIZE.sm} />Add model
    </Button>
    <div class="after">{@render lineList()}</div>
  {:else}
    <div class="title"><Sparkles size={ICON_SIZE.md} />Pick a model for the agent</div>
    <p class="msg">
      {modelState === "stale_agent_model"
        ? "The agent's model no longer exists."
        : "The agent has no model."}
    </p>
    <div class="pick">
      <SelectDropdown bind:value={picked} options={pickOptions} ariaLabel="Agent model" />
      <Button variant="primary" disabled={!pickedIsText || applying} onclick={useModel}>Use</Button>
    </div>
    {#if writeError}<p class="error">{writeError}</p>{/if}
    <div class="alt">
      Also becomes the Chat model, which the agent follows. Or
      <button class="link" onclick={() => navigation.navigateTo("models:new")}>add another model</button>.
    </div>
    <div class="after">{@render lineList()}</div>
  {/if}
</div>

{#snippet lineList()}
  <ul class="lines">
    {#each lines as line (line.text)}
      <li><line.icon size={ICON_SIZE.sm} />{line.text}</li>
    {/each}
  </ul>
{/snippet}

<style>
  .island {
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-xl);
    padding: var(--space-6) var(--space-7);
  }

  .title {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
    font-weight: 500;
    color: var(--text-secondary);
  }

  .title :global(svg) {
    color: var(--accent);
  }

  .msg {
    margin: 0 0 var(--space-4);
    font-size: var(--font-size-md);
    color: var(--text-muted);
  }

  .pick {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .pick :global(.trigger) {
    flex: 1;
    min-width: 0;
  }

  .error {
    margin: var(--space-3) 0 0;
    font-size: var(--font-size-md);
    color: var(--danger);
  }

  .alt {
    margin-top: var(--space-4);
    font-size: var(--font-size-md);
    color: var(--text-muted);
  }

  .link {
    padding: 0;
    background: none;
    border: none;
    font: inherit;
    color: var(--accent);
    cursor: pointer;
  }

  .after {
    margin-top: var(--space-6);
    padding-top: var(--space-5);
    border-top: 1px solid var(--border-faint);
  }

  .after .lines {
    margin-bottom: 0;
  }

  .lines {
    list-style: none;
    margin: 0 0 var(--space-6);
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--font-size-md);
    color: var(--text-muted);
  }

  .lines li {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .lines :global(svg) {
    flex-shrink: 0;
    color: var(--text-faint);
  }

  .chips {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .chip {
    text-align: left;
    padding: var(--space-2) var(--space-4);
    background: transparent;
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-lg);
    color: var(--text-muted);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
  }

  .chip:hover {
    border-color: var(--accent-border);
    color: var(--text-primary);
    background: var(--accent-bg-soft);
  }
</style>
