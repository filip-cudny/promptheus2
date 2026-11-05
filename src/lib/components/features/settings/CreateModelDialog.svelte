<script lang="ts">
  import { onMount, untrack } from "svelte";
  import Modal from "$lib/components/shared/ui/Modal.svelte";
  import EnvRefChip, { parseEnvRef } from "./EnvRefChip.svelte";
  import { addModel } from "$lib/services/settings";
  import { checkEnvVar } from "$lib/services/settingsDialog";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { generateId } from "$lib/utils/id";
  import { buildNewModel, newModelErrors } from "$lib/utils/newModel";
  import type { ModelType, Provider } from "$lib/types";

  let {
    modelType,
    onClose,
    onCreated,
  }: {
    modelType: ModelType;
    onClose: () => void;
    onCreated: (id: string) => void;
  } = $props();

  const store = getSettingsStore();
  const id = generateId();

  const PROVIDER_OPTIONS: { value: Provider | ""; label: string }[] = [
    { value: "", label: "None" },
    { value: "openai", label: "OpenAI" },
    { value: "anthropic", label: "Anthropic" },
    { value: "gemini", label: "Gemini" },
    { value: "elevenlabs", label: "ElevenLabs" },
  ];

  const initialType = untrack(() => modelType);
  let type = $state<ModelType>(initialType);
  let provider = $state<Provider | "">(initialType === "text" ? "openai" : "");
  let displayName = $state("");
  let model = $state("");
  let apiKey = $state("");
  let baseUrl = $state("");
  let touched = $state<Record<string, boolean>>({});
  let submitting = $state(false);
  let envResolved = $state(false);
  let displayNameEl = $state<HTMLInputElement | undefined>(undefined);

  const draft = $derived({
    type,
    display_name: displayName,
    provider: provider || null,
    model,
    api_key: apiKey,
    base_url: baseUrl,
  });
  const errors = $derived(newModelErrors(draft));
  const envVarName = $derived(parseEnvRef(apiKey));

  onMount(() => {
    displayNameEl?.focus();
  });

  $effect(() => {
    const name = envVarName;
    if (!name) {
      envResolved = false;
      return;
    }
    let cancelled = false;
    checkEnvVar(name).then((ok) => {
      if (!cancelled) envResolved = ok;
    });
    return () => {
      cancelled = true;
    };
  });

  function errorFor(field: string): string | undefined {
    return (
      store.fieldError(`models[${id}].${field}`) ??
      (touched[field] ? errors[field] : undefined)
    );
  }

  function touch(field: string) {
    touched[field] = true;
  }

  function selectType(next: ModelType) {
    type = next;
    provider = next === "text" ? "openai" : "";
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (submitting || Object.keys(errors).length > 0) return;
    submitting = true;
    try {
      await addModel(buildNewModel(draft, id));
      await store.refresh();
      onCreated(id);
      onClose();
    } catch {
      return;
    } finally {
      submitting = false;
    }
  }
</script>

<Modal title="Create model" {onClose}>
  <form class="form" onsubmit={handleSubmit}>
    <div class="field">
      <span class="label">Type</span>
      <div class="segmented">
        <button type="button" class:active={type === "text"} onclick={() => selectType("text")}>Text</button>
        <button type="button" class:active={type === "stt"} onclick={() => selectType("stt")}>STT</button>
      </div>
    </div>

    <div class="field">
      <label for="cmd-name">Display name</label>
      <input
        id="cmd-name"
        type="text"
        bind:value={displayName}
        bind:this={displayNameEl}
        onblur={() => touch("display_name")}
        class:error={errorFor("display_name")}
        autocomplete="off"
      />
      {#if errorFor("display_name")}<span class="hint err">{errorFor("display_name")}</span>{/if}
    </div>

    <div class="field">
      <label for="cmd-provider">Provider</label>
      <select id="cmd-provider" bind:value={provider}>
        {#each PROVIDER_OPTIONS as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
    </div>

    <div class="field">
      <label for="cmd-model">Model</label>
      <input
        id="cmd-model"
        type="text"
        placeholder="gpt-4.1-mini"
        bind:value={model}
        onblur={() => touch("model")}
        class:error={errorFor("model")}
        spellcheck="false"
        autocomplete="off"
      />
      {#if errorFor("model")}<span class="hint err">{errorFor("model")}</span>{/if}
    </div>

    <div class="field">
      <label for="cmd-key">API key</label>
      <input
        id="cmd-key"
        type="password"
        placeholder={"sk-… or ${ENV_VAR}"}
        bind:value={apiKey}
        onblur={() => touch("api_key")}
        class:error={errorFor("api_key")}
        spellcheck="false"
        autocomplete="off"
      />
      {#if envVarName}<EnvRefChip varName={envVarName} resolved={envResolved} />{/if}
      {#if errorFor("api_key")}<span class="hint err">{errorFor("api_key")}</span>{/if}
    </div>

    <div class="field">
      <label for="cmd-url">Base URL</label>
      <input
        id="cmd-url"
        type="text"
        placeholder="Optional"
        bind:value={baseUrl}
        onblur={() => touch("base_url")}
        class:error={errorFor("base_url")}
        spellcheck="false"
        autocomplete="off"
      />
      {#if errorFor("base_url")}<span class="hint err">{errorFor("base_url")}</span>{/if}
    </div>

    <footer>
      <button type="button" class="cancel" onclick={onClose}>Cancel</button>
      <button
        type="submit"
        class="primary"
        disabled={Object.keys(errors).length > 0 || submitting}
      >
        {submitting ? "Creating…" : "Create"}
      </button>
    </footer>
  </form>
</Modal>

<style>
  .form {
    display: contents;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .field label,
  .label {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  input[type="text"],
  input[type="password"],
  select {
    padding: var(--space-3) var(--space-4);
    background: var(--surface-sunken);
    border: 1px solid var(--border-hard);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font: inherit;
    font-size: var(--font-size-md);
  }

  input.error {
    border-color: var(--danger);
  }

  .segmented {
    display: inline-flex;
    background: var(--surface-sunken);
    border: 1px solid var(--border-hard);
    border-radius: 5px;
    padding: var(--space-1);
    width: fit-content;
  }

  .segmented button {
    padding: var(--space-2) var(--space-6);
    background: transparent;
    border: none;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
    border-radius: var(--radius-sm);
  }

  .segmented button.active {
    background: var(--accent-bg-soft);
    color: var(--accent);
  }

  .hint {
    font-size: var(--font-size-xs);
    color: var(--text-faint);
  }

  .hint.err {
    color: var(--danger);
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-3);
    padding-top: var(--space-2);
  }

  .cancel,
  .primary {
    padding: var(--space-2) var(--space-5);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-hard-2);
    background: var(--surface-elevated);
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
  }

  .primary {
    background: var(--accent);
    color: var(--text-inverted, #0a0a0a);
    border-color: var(--accent);
  }

  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
