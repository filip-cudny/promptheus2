<script lang="ts">
  import type {
    KnownModelParameterKey,
    ModelCapabilities,
    ModelParameters,
  } from "$lib/types";
  import {
    REASONING_LEVELS,
    REASONING_LEVEL_LABELS,
    type ReasoningLevel,
  } from "$lib/constants/models";

  let {
    parameters,
    capabilities = null,
    inherited = null,
    onChange,
  }: {
    parameters: ModelParameters | null;
    capabilities?: ModelCapabilities | null;
    inherited?: ModelParameters | null;
    onChange: (
      key: KnownModelParameterKey,
      value: number | string | null,
      immediate?: boolean,
    ) => void;
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

  let effortSupported = $derived(capabilities?.reasoning.kind === "effort");

  let reasoningOptions = $derived.by<ReasoningLevel[]>(() => {
    const reasoning = capabilities?.reasoning;
    const allowed =
      reasoning?.kind === "effort"
        ? (reasoning.allowed as ReasoningLevel[])
        : [...REASONING_LEVELS];
    return allowed.includes("none") ? allowed : ["none", ...allowed];
  });

  let inheritedEffort = $derived<string | null>(
    typeof inherited?.reasoning_effort === "string" && inherited.reasoning_effort.length > 0
      ? inherited.reasoning_effort
      : null,
  );

  let reasoningDefault = $derived.by<string>(() => {
    if (inheritedEffort && reasoningOptions.includes(inheritedEffort as ReasoningLevel)) {
      return inheritedEffort;
    }
    if (reasoningOptions.includes("medium")) return "medium";
    return reasoningOptions.find((o) => o !== "none") ?? reasoningOptions[0] ?? "medium";
  });

  function isOverridden(key: KnownModelParameterKey): boolean {
    if (!parameters) return false;
    return parameters[key] !== null && parameters[key] !== undefined;
  }

  function inheritedNumber(key: KnownModelParameterKey): number | null {
    const v = inherited?.[key];
    return typeof v === "number" && Number.isFinite(v) ? v : null;
  }

  function toggleSliderOverride(slider: Slider) {
    if (isOverridden(slider.key)) {
      onChange(slider.key, null);
    } else {
      onChange(slider.key, inheritedNumber(slider.key) ?? slider.default);
    }
  }

  function toggleMaxTokens() {
    if (isOverridden("max_tokens")) {
      onChange("max_tokens", null);
    } else {
      onChange("max_tokens", inheritedNumber("max_tokens") ?? 4096);
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
    return typeof v === "string" && v.length > 0 ? v : fallback;
  }

  function effortLabel(value: string): string {
    return REASONING_LEVEL_LABELS[value as ReasoningLevel] ?? value;
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
        {:else if inheritedNumber(slider.key) !== null}
          <span class="inherit-hint">inherits {inheritedNumber(slider.key)}</span>
        {/if}
      </div>
      {#if overridden}
        <input
          type="range"
          min={slider.min}
          max={slider.max}
          step={slider.step}
          {value}
          oninput={(e) =>
            onChange(slider.key, Number((e.target as HTMLInputElement).value), false)}
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
        {#if !overridden && inheritedNumber("max_tokens") !== null}
          <span class="inherit-hint">inherits {inheritedNumber("max_tokens")}</span>
        {/if}
      </div>
      {#if overridden}
        <input
          type="number"
          min="1"
          value={getNumber("max_tokens", 4096)}
          oninput={(e) => {
            const n = Number((e.target as HTMLInputElement).value);
            onChange("max_tokens", Number.isFinite(n) && n >= 1 ? n : 1, false);
          }}
        />
      {/if}
    </div>
  {/if}

  {#if effortSupported || isOverridden("reasoning_effort")}
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
        {#if !overridden && inheritedEffort}
          <span class="inherit-hint">inherits {effortLabel(inheritedEffort)}</span>
        {/if}
      </div>
      {#if overridden}
        <select
          {value}
          onchange={(e) =>
            onChange("reasoning_effort", (e.currentTarget as HTMLSelectElement).value)}
        >
          {#if !reasoningOptions.includes(value as ReasoningLevel)}
            <option value={value}>{effortLabel(value)} — not accepted by this model</option>
          {/if}
          {#each reasoningOptions as opt (opt)}
            <option value={opt}>{effortLabel(opt)}</option>
          {/each}
        </select>
        {#if !effortSupported}
          <p class="param-warn">
            The effective model does not advertise reasoning effort — this value is dropped
            when the request is built. Uncheck to remove it from the file.
          </p>
        {/if}
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

  .inherit-hint {
    font-size: var(--font-size-xs);
    color: var(--text-disabled);
  }

  .param-warn {
    font-size: var(--font-size-sm);
    color: var(--warning);
    margin: 0;
    line-height: 1.5;
  }

  input[type="range"] {
    width: 100%;
  }

  input[type="number"],
  select {
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
