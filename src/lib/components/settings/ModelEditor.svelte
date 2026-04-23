<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import { Eye, EyeOff, Copy, Trash2, CopyPlus } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import EnvRefChip, { parseEnvRef } from "./EnvRefChip.svelte";
  import ParametersKnown from "./ParametersKnown.svelte";
  import ParametersCustom, {
    entriesFromExtra,
    entriesToExtra,
    type CustomParamEntry,
  } from "./ParametersCustom.svelte";
  import {
    addModel,
    deleteModel,
    updateModel,
  } from "$lib/services/settings";
  import { checkEnvVar } from "$lib/services/settingsDialog";
  import { generateId } from "$lib/utils/id";
  import {
    KNOWN_MODEL_PARAMETER_KEYS,
    type ApiMode,
    type KnownModelParameterKey,
    type ModelConfig,
    type ModelParameters,
    type ModelType,
    type Provider,
    type SurfaceKind,
  } from "$lib/types";

  let {
    model,
    referencedSurface,
    debounceMs,
    onDeleted,
    onDuplicated,
  }: {
    model: ModelConfig;
    referencedSurface: SurfaceKind | null;
    debounceMs: number;
    onDeleted: () => void;
    onDuplicated: (id: string) => void;
  } = $props();

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

  const API_MODE_OPTIONS: { value: ApiMode | ""; label: string }[] = [
    { value: "", label: "Auto" },
    { value: "responses", label: "Responses" },
    { value: "completions", label: "Completions" },
  ];

  const baseUrlPlaceholder = $derived(
    draft.provider === "openai" ? "https://api.openai.com/v1" : "https://…",
  );

  const showApiMode = $derived(draft.type === "text" && draft.provider === "openai");
  const isText = $derived(draft.type === "text");
</script>

<div class="model-editor">
  <header class="editor-header">
    <h1>{draft.display_name || "Untitled model"}</h1>
    {#if referencedSurface}
      <span class="badge">in use by {referencedSurface}</span>
    {/if}
  </header>

  <section class="card">
    <h3>Basic</h3>

    <div class="field">
      <label for="display_name">Display name</label>
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
      {#if validationErrors.display_name}
        <div class="field-error">{validationErrors.display_name}</div>
      {/if}
    </div>

    <div class="field">
      <label for="model_id">Model</label>
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
      {#if validationErrors.model}
        <div class="field-error">{validationErrors.model}</div>
      {/if}
    </div>

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

    <div class="field">
      <label for="group">Group</label>
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
    </div>
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

    <div class="field">
      <label for="base_url">Base URL</label>
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
    </div>

    {#if showApiMode}
      <div class="field">
        <label for="api_mode">API mode</label>
        <select
          id="api_mode"
          value={draft.api_mode ?? ""}
          onchange={(e) => {
            const v = (e.target as HTMLSelectElement).value;
            draft.api_mode = (v as ApiMode) || null;
            scheduleSave(true);
          }}
        >
          {#each API_MODE_OPTIONS as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
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
      <div class="field">
        <label for="ctx_window">Context window size</label>
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
      </div>
    </section>

    <section class="card">
      <h3>Parameters</h3>
      <h4>Known</h4>
      <ParametersKnown
        parameters={draft.parameters}
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
            {#if referencedSurface}
              <br />
              <span class="warn">It is currently in use by surface "{referencedSurface}".</span>
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
    padding: 18px 24px 32px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 720px;
  }

  .editor-header {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .editor-header h1 {
    font-size: 18px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.95);
    margin: 0;
  }

  .badge {
    font-size: 10px;
    padding: 2px 6px;
    background: rgba(217, 179, 74, 0.15);
    color: #d9b34a;
    border: 1px solid rgba(217, 179, 74, 0.35);
    border-radius: 8px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }

  .card {
    background: #232323;
    border: 1px solid #2e2e2e;
    border-radius: 6px;
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .card.danger {
    border-color: rgba(217, 115, 115, 0.3);
  }

  h3 {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: rgba(255, 255, 255, 0.45);
    margin: 0;
  }

  h4 {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: rgba(255, 255, 255, 0.4);
    margin: 4px 0 0;
  }

  h4.custom-heading {
    margin-top: 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    padding-top: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field > label,
  .field-label {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.7);
  }

  input[type="text"],
  input[type="password"],
  input[type="number"],
  select {
    width: 100%;
    padding: 6px 8px;
    background: #1a1a1a;
    border: 1px solid #3a3a3a;
    border-radius: 4px;
    color: rgba(255, 255, 255, 0.92);
    font: inherit;
    font-size: 12px;
  }

  input.error {
    border-color: #d97373;
  }

  .field-error {
    font-size: 11px;
    color: #d97373;
  }

  .segmented {
    display: inline-flex;
    background: #1a1a1a;
    border: 1px solid #3a3a3a;
    border-radius: 5px;
    padding: 2px;
    width: fit-content;
  }

  .segmented button {
    padding: 4px 12px;
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.65);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    border-radius: 3px;
  }

  .segmented button.active {
    background: rgba(91, 141, 217, 0.2);
    color: #8db3ee;
  }

  .api-key-row {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 5px 8px;
    background: transparent;
    border: 1px solid #3a3a3a;
    border-radius: 4px;
    color: rgba(255, 255, 255, 0.6);
    font: inherit;
    font-size: 11px;
    cursor: pointer;
  }

  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.04);
    color: rgba(255, 255, 255, 0.85);
  }

  .toggle-field label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: rgba(255, 255, 255, 0.85);
    cursor: pointer;
  }

  .toggle-field input[type="checkbox"] {
    accent-color: #5b8dd9;
  }

  .helper {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.4);
    margin-top: 2px;
  }

  .danger-row {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .delete-btn,
  .duplicate-btn {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: transparent;
    border: 1px solid #3a3a3a;
    border-radius: 5px;
    color: rgba(255, 255, 255, 0.78);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .duplicate-btn:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .delete-btn {
    color: #d97373;
    border-color: rgba(217, 115, 115, 0.4);
  }

  .delete-btn:hover {
    background: rgba(217, 115, 115, 0.08);
  }

  .delete-confirm p {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.85);
    margin: 0 0 8px;
  }

  .delete-confirm .warn {
    color: #d9b34a;
    font-size: 11px;
  }

  .confirm-actions {
    display: flex;
    gap: 8px;
  }

  .confirm-actions button {
    padding: 5px 10px;
    background: #2a2a2a;
    border: 1px solid #3e3e3e;
    border-radius: 4px;
    color: rgba(255, 255, 255, 0.78);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .confirm-actions .confirm {
    background: rgba(217, 115, 115, 0.15);
    color: #ffa3a3;
    border-color: rgba(217, 115, 115, 0.5);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.4);
    padding: 8px 0 0;
  }

  .meta code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.6);
  }
</style>
