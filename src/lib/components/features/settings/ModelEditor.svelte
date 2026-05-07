<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import { Eye, EyeOff, Copy, Trash2, CopyPlus } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { formatSurfaceList } from "$lib/constants/surfaces";
  import EnvRefChip, { parseEnvRef } from "./EnvRefChip.svelte";
  import ParametersKnown from "./ParametersKnown.svelte";
  import CapabilitiesEditor from "./CapabilitiesEditor.svelte";
  import ParametersCustom, {
    entriesFromExtra,
    entriesToExtra,
    type CustomParamEntry,
  } from "./ParametersCustom.svelte";
  import FormRow from "$lib/components/shared/ui/FormRow.svelte";
  import {
    addModel,
    deleteModel,
    updateModel,
  } from "$lib/services/settings";
  import {
    prefetchCapabilities,
    getCachedCapabilities,
  } from "$lib/stores/capabilities.svelte";
  import { checkEnvVar } from "$lib/services/settingsDialog";
  import { generateId } from "$lib/utils/id";
  import {
    KNOWN_MODEL_PARAMETER_KEYS,
    type ApiMode,
    type KnownModelParameterKey,
    type ModelCapabilities,
    type ModelConfig,
    type ModelParameters,
    type ModelType,
    type Provider,
    type SurfaceKind,
  } from "$lib/types";

  let {
    model,
    surfaces,
    debounceMs,
    onDeleted,
    onDuplicated,
  }: {
    model: ModelConfig;
    surfaces: SurfaceKind[];
    debounceMs: number;
    onDeleted: () => void;
    onDuplicated: (id: string) => void;
  } = $props();

  const surfacesLabel = $derived(formatSurfaceList(surfaces));

  let draft = $state<ModelConfig>(untrack(() => structuredClone(model)));
  let customEntries = $state<CustomParamEntry[]>(
    untrack(() => entriesFromExtra(extraOf(model.parameters))),
  );
  let customErrors = $state<Record<string, string>>({});
  let validationErrors = $state<Record<string, string>>({});
  let envResolved = $state<boolean>(false);
  let envVarName = $state<string | null>(null);
  let showApiKey = $state(false);
  let confirmDelete = $state(false);
  let copiedId = $state(false);

  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const m = model;
    untrack(() => {
      draft = structuredClone(m);
      customEntries = entriesFromExtra(extraOf(m.parameters));
      customErrors = {};
      validationErrors = {};
    });
  });

  $effect(() => {
    const apiKey = draft.api_key;
    const name = parseEnvRef(apiKey);
    envVarName = name;
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

  onDestroy(() => {
    if (saveTimer) clearTimeout(saveTimer);
  });

  function extraOf(params: ModelParameters | null | undefined): Record<string, unknown> {
    if (!params) return {};
    const out: Record<string, unknown> = {};
    const known = new Set<string>(KNOWN_MODEL_PARAMETER_KEYS);
    for (const [k, v] of Object.entries(params)) {
      if (known.has(k)) continue;
      out[k] = v;
    }
    return out;
  }

  function buildParametersToSend(): ModelParameters | null {
    const customResult = entriesToExtra(customEntries);
    customErrors = customResult.errors;

    const known: Partial<Record<KnownModelParameterKey, number | string | null>> = {
      temperature: numericOrNull(draft.parameters?.temperature),
      max_tokens: numericOrNull(draft.parameters?.max_tokens),
      top_p: numericOrNull(draft.parameters?.top_p),
      frequency_penalty: numericOrNull(draft.parameters?.frequency_penalty),
      presence_penalty: numericOrNull(draft.parameters?.presence_penalty),
      reasoning_effort: stringOrNull(draft.parameters?.reasoning_effort),
    };

    const allEmpty =
      Object.values(known).every((v) => v === null || v === undefined) &&
      Object.keys(customResult.extra).length === 0;
    if (allEmpty) return null;

    const merged: ModelParameters = {
      temperature: known.temperature as number | null,
      max_tokens: known.max_tokens as number | null,
      top_p: known.top_p as number | null,
      frequency_penalty: known.frequency_penalty as number | null,
      presence_penalty: known.presence_penalty as number | null,
      reasoning_effort: known.reasoning_effort as string | null,
      ...customResult.extra,
    };
    return merged;
  }

  function numericOrNull(v: unknown): number | null {
    return typeof v === "number" && Number.isFinite(v) ? v : null;
  }

  function stringOrNull(v: unknown): string | null {
    return typeof v === "string" && v.length > 0 ? v : null;
  }

  function validate(): boolean {
    const errs: Record<string, string> = {};
    if (!draft.display_name.trim()) errs.display_name = "Required";
    if (!draft.model.trim()) errs.model = "Required";
    validationErrors = errs;
    return Object.keys(errs).length === 0;
  }

  async function persist() {
    if (!validate()) return;
    const parameters = buildParametersToSend();
    if (Object.keys(customErrors).length > 0) return;

    const config: ModelConfig = {
      ...draft,
      parameters,
    };
    try {
      await updateModel(model.id, config);
    } catch (e) {
      console.error("update_model failed", e);
    }
  }

  function scheduleSave(immediate: boolean) {
    if (saveTimer) clearTimeout(saveTimer);
    if (immediate) {
      persist();
    } else {
      saveTimer = setTimeout(persist, debounceMs);
    }
  }

  function setKnownParameter(key: KnownModelParameterKey, value: number | string | null) {
    const params: ModelParameters =
      draft.parameters ?? {
        temperature: null,
        max_tokens: null,
        top_p: null,
        frequency_penalty: null,
        presence_penalty: null,
        reasoning_effort: null,
      };
    draft.parameters = { ...params, [key]: value as never };
    scheduleSave(true);
  }

  function setCapabilities(next: ModelCapabilities | null) {
    draft.capabilities = next;
    scheduleSave(true);
  }

  $effect(() => {
    customEntries;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(persist, debounceMs);
  });

  async function handleDelete() {
    try {
      await deleteModel(model.id);
      onDeleted();
    } catch (e) {
      console.error("delete_model failed", e);
    }
  }

  async function handleDuplicate() {
    const copy: ModelConfig = {
      ...structuredClone(draft),
      id: generateId(),
      display_name: `${draft.display_name || "Untitled"} (copy)`,
    };
    try {
      await addModel(copy);
      onDuplicated(copy.id);
    } catch (e) {
      console.error("duplicate failed", e);
    }
  }

  async function copyId() {
    try {
      await navigator.clipboard.writeText(model.id);
      copiedId = true;
      setTimeout(() => (copiedId = false), 1200);
    } catch {
      copiedId = false;
    }
  }

  const PROVIDER_OPTIONS: { value: Provider | ""; label: string }[] = [
    { value: "", label: "None" },
    { value: "openai", label: "OpenAI" },
    { value: "anthropic", label: "Anthropic" },
    { value: "gemini", label: "Gemini" },
    { value: "elevenlabs", label: "ElevenLabs" },
  ];

  const TYPE_OPTIONS: { value: ModelType; label: string }[] = [
    { value: "text", label: "Text" },
    { value: "stt", label: "STT" },
  ];

  const API_MODE_OPTIONS: { value: ApiMode; label: string }[] = [
    { value: "responses", label: "Responses (/v1/responses)" },
    { value: "completions", label: "Chat Completions (/v1/chat/completions)" },
  ];

  const baseUrlPlaceholder = $derived(
    draft.provider === "openai" ? "https://api.openai.com/v1" : "https://…",
  );

  const showApiMode = $derived(draft.type === "text" && draft.provider === "openai");
  const isText = $derived(draft.type === "text");

  $effect(() => {
    prefetchCapabilities(model);
  });

  const resolvedCapabilities = $derived(
    draft.capabilities ?? getCachedCapabilities(model),
  );
</script>

<div class="model-editor">
  <header class="editor-header">
    <h1>{draft.display_name || "Untitled model"}</h1>
    {#if surfaces.length > 0}
      <span class="badge">in use by {surfacesLabel}</span>
    {/if}
  </header>

  <section class="card">
    <h3>Basic</h3>

    <FormRow label="Display name" error={validationErrors.display_name}>
      <input
        id="display_name"
        type="text"
        class:error={validationErrors.display_name}
        value={draft.display_name}
        oninput={(e) => {
          draft.display_name = (e.target as HTMLInputElement).value;
          scheduleSave(false);
        }}
      />
    </FormRow>

    <FormRow label="Model" error={validationErrors.model}>
      <input
        id="model_id"
        type="text"
        placeholder="gpt-4.1-mini"
        class:error={validationErrors.model}
        value={draft.model}
        oninput={(e) => {
          draft.model = (e.target as HTMLInputElement).value;
          scheduleSave(false);
        }}
      />
    </FormRow>

    <div class="field">
      <span class="field-label">Type</span>
      <div class="segmented">
        {#each TYPE_OPTIONS as opt}
          <button
            class:active={draft.type === opt.value}
            onclick={() => {
              if (draft.type === opt.value) return;
              draft.type = opt.value;
              scheduleSave(true);
            }}
          >
            {opt.label}
          </button>
        {/each}
      </div>
    </div>

    <div class="field">
      <label for="provider">Provider</label>
      <select
        id="provider"
        value={draft.provider ?? ""}
        onchange={(e) => {
          const v = (e.target as HTMLSelectElement).value;
          draft.provider = (v as Provider) || null;
          scheduleSave(true);
        }}
      >
        {#each PROVIDER_OPTIONS as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
    </div>

    <FormRow label="Group">
      <input
        id="group"
        type="text"
        placeholder="Optional grouping"
        value={draft.group ?? ""}
        oninput={(e) => {
          const v = (e.target as HTMLInputElement).value;
          draft.group = v || null;
          scheduleSave(false);
        }}
      />
    </FormRow>
  </section>

  <section class="card">
    <h3>Connection</h3>

    <div class="field">
      <label for="api_key">API key</label>
      <div class="api-key-row">
        <input
          id="api_key"
          type={showApiKey ? "text" : "password"}
          placeholder={'sk-… or ${ENV_VAR}'}
          value={draft.api_key ?? ""}
          oninput={(e) => {
            const v = (e.target as HTMLInputElement).value;
            draft.api_key = v || null;
            scheduleSave(false);
          }}
        />
        <button
          class="icon-btn"
          title={showApiKey ? "Hide" : "Show"}
          onclick={() => (showApiKey = !showApiKey)}
        >
          {#if showApiKey}
            <EyeOff size={ICON_SIZE.md} />
          {:else}
            <Eye size={ICON_SIZE.md} />
          {/if}
        </button>
      </div>
      {#if envVarName}
        <EnvRefChip varName={envVarName} resolved={envResolved} />
      {/if}
    </div>

    <FormRow label="Base URL">
      <input
        id="base_url"
        type="text"
        placeholder={baseUrlPlaceholder}
        value={draft.base_url ?? ""}
        oninput={(e) => {
          const v = (e.target as HTMLInputElement).value;
          draft.base_url = v || null;
          scheduleSave(false);
        }}
      />
    </FormRow>

    {#if showApiMode}
      <div class="field">
        <label for="api_mode">API mode</label>
        <select
          id="api_mode"
          value={draft.api_mode ?? "responses"}
          onchange={(e) => {
            draft.api_mode = (e.target as HTMLSelectElement).value as ApiMode;
            scheduleSave(true);
          }}
        >
          {#each API_MODE_OPTIONS as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
        <p class="helper">
          Which OpenAI-compatible endpoint to call. <strong>Responses</strong> is required for
          built-in tools like web search and reasoning summaries on GPT-5/o-series. Switch to
          <strong>Chat Completions</strong> if the Base URL points to a provider that only
          implements <code>/v1/chat/completions</code> (most OpenAI-compatible gateways: OpenRouter,
          vLLM, Ollama, Together, etc.).
        </p>
      </div>
    {/if}

    <div class="field toggle-field">
      <label>
        <input
          type="checkbox"
          checked={draft.store}
          onchange={(e) => {
            draft.store = (e.target as HTMLInputElement).checked;
            scheduleSave(true);
          }}
        />
        <span>Store request/response on provider</span>
      </label>
      <p class="helper">
        When off, the provider won't retain request/response for training or logs (OpenAI-specific).
      </p>
    </div>
  </section>

  {#if isText}
    <section class="card">
      <h3>Capabilities</h3>
      <FormRow label="Context window size">
        <input
          id="ctx_window"
          type="number"
          placeholder="auto from model"
          min="1"
          value={draft.context_window_size ?? ""}
          oninput={(e) => {
            const v = (e.target as HTMLInputElement).value;
            const n = v ? Number(v) : null;
            draft.context_window_size =
              n !== null && Number.isFinite(n) && n > 0 ? Math.floor(n) : null;
            scheduleSave(false);
          }}
        />
      </FormRow>
      <CapabilitiesEditor
        capabilities={draft.capabilities ?? null}
        onChange={setCapabilities}
      />
    </section>

    <section class="card">
      <h3>Parameters</h3>
      <h4>Known</h4>
      <ParametersKnown
        parameters={draft.parameters}
        capabilities={resolvedCapabilities}
        onChange={setKnownParameter}
      />
      <h4 class="custom-heading">Custom</h4>
      <ParametersCustom bind:entries={customEntries} errors={customErrors} />
    </section>
  {/if}

  <section class="card danger">
    <h3>Danger zone</h3>
    <div class="danger-row">
      <button class="duplicate-btn" onclick={handleDuplicate}>
        <CopyPlus size={ICON_SIZE.md} />
        <span>Duplicate model</span>
      </button>
      {#if !confirmDelete}
        <button class="delete-btn" onclick={() => (confirmDelete = true)}>
          <Trash2 size={ICON_SIZE.md} />
          <span>Delete model</span>
        </button>
      {:else}
        <div class="delete-confirm">
          <p>
            Delete <strong>{draft.display_name || "this model"}</strong>?
            {#if surfaces.length > 0}
              <br />
              <span class="warn">
                It is currently in use by
                {surfaces.length === 1 ? "surface" : "surfaces"}
                "{surfacesLabel}".
              </span>
            {/if}
          </p>
          <div class="confirm-actions">
            <button onclick={() => (confirmDelete = false)}>Cancel</button>
            <button class="delete-btn confirm" onclick={handleDelete}>
              Yes, delete
            </button>
          </div>
        </div>
      {/if}
    </div>
  </section>

  <footer class="meta">
    <span>ID: <code>{model.id}</code></span>
    <button class="icon-btn" onclick={copyId} title="Copy ID">
      <Copy size={ICON_SIZE.sm} />
      <span>{copiedId ? "Copied" : "Copy"}</span>
    </button>
  </footer>
</div>

<style>
  .model-editor {
    padding: 18px var(--space-12) var(--space-16);
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
    max-width: 720px;
  }

  .editor-header {
    display: flex;
    align-items: center;
    gap: var(--space-5);
  }

  .editor-header h1 {
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    margin: var(--space-0);
    line-height: 1.2;
  }

  .badge {
    font-size: var(--font-size-xs);
    padding: var(--space-1) var(--space-3);
    background: var(--accent-bg-soft);
    color: var(--accent);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius-xl);
    text-transform: uppercase;
    letter-spacing: 0.4px;
    line-height: 1.4;
  }

  .card {
    background: var(--surface-base);
    border: 1px solid var(--border-hard);
    border-radius: var(--radius-lg);
    padding: var(--space-7) var(--space-8);
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  .card.danger {
    border-color: var(--danger-border);
  }

  h3 {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--text-muted);
    margin: var(--space-0);
  }

  h4 {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--text-disabled);
    margin: var(--space-2) var(--space-0) var(--space-0);
  }

  h4.custom-heading {
    margin-top: var(--space-6);
    border-top: 1px solid var(--border-faint);
    padding-top: var(--space-6);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .field > label,
  .field-label {
    font-size: var(--font-size-md);
    color: var(--text-secondary);
  }

  input[type="text"],
  input[type="password"],
  input[type="number"],
  select {
    width: 100%;
    padding: var(--space-3) var(--space-4);
    background-color: var(--surface-sunken);
    border: 1px solid var(--border-hard);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font: inherit;
    font-size: var(--font-size-md);
  }

  select {
    padding-right: var(--space-8);
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

  .api-key-row {
    display: flex;
    gap: var(--space-3);
    align-items: center;
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 5px var(--space-4);
    background: transparent;
    border: 1px solid var(--border-hard);
    border-radius: var(--radius-md);
    color: var(--text-muted);
    font: inherit;
    font-size: var(--font-size-sm);
    cursor: pointer;
  }

  .icon-btn:hover {
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
  }

  .toggle-field label {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3);
    color: var(--text-primary);
    cursor: pointer;
  }

  .helper {
    font-size: var(--font-size-sm);
    color: var(--text-disabled);
    margin-top: var(--space-1);
    line-height: 1.5;
  }

  .helper code {
    font-family: var(--font-mono);
    font-size: 0.92em;
    padding: 0 4px;
    background: var(--surface-overlay-faint);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
  }

  .helper strong {
    color: var(--text-secondary);
    font-weight: var(--font-weight-semibold);
  }

  .danger-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .delete-btn,
  .duplicate-btn {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-6);
    background: transparent;
    border: 1px solid var(--border-hard);
    border-radius: 5px;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
  }

  .duplicate-btn:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .delete-btn {
    color: var(--danger);
    border-color: var(--danger-border);
  }

  .delete-btn:hover {
    background: var(--danger-bg-soft);
  }

  .delete-confirm p {
    font-size: var(--font-size-md);
    color: var(--text-primary);
    margin: var(--space-0) var(--space-0) var(--space-4);
  }

  .delete-confirm .warn {
    color: var(--warning);
    font-size: var(--font-size-sm);
  }

  .confirm-actions {
    display: flex;
    gap: var(--space-4);
  }

  .confirm-actions button {
    padding: 5px var(--space-5);
    background: var(--surface-elevated);
    border: 1px solid var(--border-hard-2);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
  }

  .confirm-actions .confirm {
    background: var(--danger-bg-soft);
    color: var(--danger);
    border-color: var(--danger-border);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    font-size: var(--font-size-sm);
    color: var(--text-disabled);
    padding: var(--space-4) var(--space-0) var(--space-0);
  }

  .meta code {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }
</style>
