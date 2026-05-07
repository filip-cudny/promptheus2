<script lang="ts">
  import type {
    KnownModelParameterKey,
    ModelCapabilities,
    ModelParameters,
  } from "$lib/types";
  import type { ReasoningLevel } from "$lib/constants/models";

  let {
    parameters,
    capabilities = null,
    onChange,
  }: {
    parameters: ModelParameters | null;
    capabilities?: ModelCapabilities | null;
    onChange: (key: KnownModelParameterKey, value: number | string | null) => void;
  } = $props();

  type Slider = {
    key: KnownModelParameterKey;
    label: string;
    min: number;
    max: number;
    step: number;
    default: number;
  };

  const SLIDERS: Slider[] = [
    { key: "temperature", label: "Temperature", min: 0, max: 2, step: 0.1, default: 0.7 },
    { key: "top_p", label: "Top P", min: 0, max: 1, step: 0.05, default: 1 },
    { key: "frequency_penalty", label: "Frequency penalty", min: -2, max: 2, step: 0.1, default: 0 },
    { key: "presence_penalty", label: "Presence penalty", min: -2, max: 2, step: 0.1, default: 0 },
  ];

  let reasoningKind = $derived(capabilities?.reasoning.kind ?? "unsupported");

  let reasoningOptions = $derived.by<ReasoningLevel[]>(() => {
    const reasoning = capabilities?.reasoning;
    if (!reasoning || reasoning.kind !== "effort") return [];
    const allowed = reasoning.allowed as ReasoningLevel[];
    return allowed.includes("none") ? allowed : ["none", ...allowed];
  });

  let reasoningDefault = $derived<string>(
    reasoningOptions.find((o) => o !== "none") ?? reasoningOptions[0] ?? "medium",
  );

  function isOverridden(key: KnownModelParameterKey): boolean {
    if (!parameters) return false;
    return parameters[key] !== null && parameters[key] !== undefined;
  }

  function toggleSliderOverride(slider: Slider) {
    if (isOverridden(slider.key)) {
      onChange(slider.key, null);
    } else {
      onChange(slider.key, slider.default);
    }
  }

  function toggleMaxTokens() {
    if (isOverridden("max_tokens")) {
      onChange("max_tokens", null);
    } else {
      onChange("max_tokens", 4096);
    }
  }

  function toggleReasoning() {
    if (isOverridden("reasoning_effort")) {
      onChange("reasoning_effort", null);
    } else {
      onChange("reasoning_effort", reasoningDefault);
    }
  }

  function getNumber(key: KnownModelParameterKey, fallback: number): number {
    const v = parameters?.[key];
    return typeof v === "number" ? v : fallback;
  }

  function getString(key: KnownModelParameterKey, fallback: string): string {
    const v = parameters?.[key];
    return typeof v === "string" ? v : fallback;
  }
</script>

<div class="known-params">
  {#each SLIDERS as slider (slider.key)}
    {@const overridden = isOverridden(slider.key)}
    {@const value = getNumber(slider.key, slider.default)}
    <div class="param">
      <div class="param-header">
        <label>
          <input
            type="checkbox"
            checked={overridden}
            onchange={() => toggleSliderOverride(slider)}
          />
          <span>{slider.label}</span>
        </label>
        {#if overridden}
          <span class="value">{value.toFixed(2)}</span>
        {/if}
      </div>
      {#if overridden}
        <input
          type="range"
          min={slider.min}
          max={slider.max}
          step={slider.step}
          {value}
          oninput={(e) => onChange(slider.key, Number((e.target as HTMLInputElement).value))}
        />
        <div class="range-meta">
          <span>{slider.min}</span>
          <span>{slider.max}</span>
        </div>
      {/if}
    </div>
  {/each}

  {#if true}
    {@const overridden = isOverridden("max_tokens")}
    <div class="param">
      <div class="param-header">
        <label>
          <input
            type="checkbox"
            checked={overridden}
            onchange={toggleMaxTokens}
          />
          <span>Max tokens</span>
        </label>
      </div>
      {#if overridden}
        <input
          type="number"
          min="1"
          value={getNumber("max_tokens", 4096)}
          oninput={(e) => {
            const n = Number((e.target as HTMLInputElement).value);
            onChange("max_tokens", Number.isFinite(n) && n >= 1 ? n : 1);
          }}
        />
      {/if}
    </div>
  {/if}

  {#if reasoningKind === "effort"}
    {@const overridden = isOverridden("reasoning_effort")}
    {@const value = getString("reasoning_effort", reasoningDefault)}
    <div class="param">
      <div class="param-header">
        <label>
          <input
            type="checkbox"
            checked={overridden}
            onchange={toggleReasoning}
          />
          <span>Reasoning effort</span>
        </label>
      </div>
      {#if overridden}
        <input
          list="reasoning-options"
          {value}
          oninput={(e) => onChange("reasoning_effort", (e.target as HTMLInputElement).value)}
        />
        <datalist id="reasoning-options">
          {#each reasoningOptions as opt}
            <option value={opt}></option>
          {/each}
        </datalist>
      {/if}
    </div>
  {/if}
</div>

<style>
  .known-params {
    display: flex;
    flex-direction: column;
    gap: var(--space-7);
  }

  .param {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .param-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }

  label {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3);
    font-size: var(--font-size-md);
    color: var(--text-primary);
    cursor: pointer;
  }

  .value {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  input[type="range"] {
    width: 100%;
  }

  input[type="number"],
  input[list] {
    width: 100%;
    padding: 5px var(--space-4);
    background: var(--surface-sunken);
    border: 1px solid var(--border-hard);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font: inherit;
    font-size: var(--font-size-md);
  }

  .range-meta {
    display: flex;
    justify-content: space-between;
    font-size: var(--font-size-xs);
    color: var(--text-disabled);
  }
</style>
