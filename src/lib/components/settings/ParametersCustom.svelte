<script lang="ts" module>
  import { KNOWN_MODEL_PARAMETER_KEYS } from "$lib/types";

  export type CustomParamType = "string" | "number" | "boolean" | "json";

  export interface CustomParamEntry {
    id: string;
    key: string;
    type: CustomParamType;
    raw: string;
  }

  const RESERVED = new Set<string>(KNOWN_MODEL_PARAMETER_KEYS);

  function inferType(value: unknown): CustomParamType {
    if (typeof value === "string") return "string";
    if (typeof value === "number") return "number";
    if (typeof value === "boolean") return "boolean";
    return "json";
  }

  function valueToRaw(value: unknown, type: CustomParamType): string {
    if (type === "string") return String(value ?? "");
    if (type === "number") return String(value ?? 0);
    if (type === "boolean") return value ? "true" : "false";
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return "";
    }
  }

  export function entriesFromExtra(
    extra: Record<string, unknown>,
  ): CustomParamEntry[] {
    return Object.entries(extra).map(([key, value], idx) => {
      const type = inferType(value);
      return {
        id: `entry-${idx}-${key}`,
        key,
        type,
        raw: valueToRaw(value, type),
      };
    });
  }

  export interface CustomParamParseResult {
    extra: Record<string, unknown>;
    errors: Record<string, string>;
  }

  export function entriesToExtra(
    entries: CustomParamEntry[],
  ): CustomParamParseResult {
    const errors: Record<string, string> = {};
    const seen = new Set<string>();
    const extra: Record<string, unknown> = {};

    for (const entry of entries) {
      const trimmed = entry.key.trim();
      if (!trimmed) {
        errors[entry.id] = "Key is required";
        continue;
      }
      if (RESERVED.has(trimmed)) {
        errors[entry.id] = "Use the known parameter above";
        continue;
      }
      if (seen.has(trimmed)) {
        errors[entry.id] = "Duplicate key";
        continue;
      }
      seen.add(trimmed);

      try {
        extra[trimmed] = parseValue(entry.type, entry.raw);
      } catch (e) {
        errors[entry.id] =
          e instanceof Error ? e.message : "Invalid value";
      }
    }

    return { extra, errors };
  }

  function parseValue(type: CustomParamType, raw: string): unknown {
    if (type === "string") return raw;
    if (type === "number") {
      const n = Number(raw);
      if (!Number.isFinite(n)) throw new Error("Invalid number");
      return n;
    }
    if (type === "boolean") return raw === "true";
    if (raw.trim() === "") throw new Error("JSON cannot be empty");
    return JSON.parse(raw);
  }
</script>

<script lang="ts">
  import { Plus, Trash2 } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";

  let {
    entries = $bindable<CustomParamEntry[]>([]),
    errors = {},
  }: {
    entries: CustomParamEntry[];
    errors?: Record<string, string>;
  } = $props();

  function addEntry() {
    const id = `entry-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    entries = [...entries, { id, key: "", type: "string", raw: "" }];
  }

  function removeEntry(id: string) {
    entries = entries.filter((e) => e.id !== id);
  }

  function updateEntry(id: string, patch: Partial<CustomParamEntry>) {
    entries = entries.map((e) =>
      e.id === id
        ? {
            ...e,
            ...patch,
            raw: patch.type && patch.type !== e.type ? convertRaw(e.raw, e.type, patch.type) : patch.raw ?? e.raw,
          }
        : e,
    );
  }

  function convertRaw(raw: string, from: CustomParamType, to: CustomParamType): string {
    if (from === to) return raw;
    if (to === "boolean") return raw === "true" ? "true" : "false";
    if (to === "number") {
      const n = Number(raw);
      return Number.isFinite(n) ? String(n) : "0";
    }
    if (to === "json") {
      try {
        if (from === "number") return raw;
        if (from === "boolean") return raw;
        return JSON.stringify(raw);
      } catch {
        return "";
      }
    }
    return raw;
  }
</script>

<div class="custom-params">
  {#each entries as entry (entry.id)}
    {@const err = errors[entry.id]}
    <div class="entry" class:has-error={!!err}>
      <div class="entry-row">
        <input
          class="key-input"
          type="text"
          placeholder="key"
          value={entry.key}
          oninput={(e) =>
            updateEntry(entry.id, { key: (e.target as HTMLInputElement).value })}
        />
        <select
          class="type-select"
          value={entry.type}
          onchange={(e) =>
            updateEntry(entry.id, {
              type: (e.target as HTMLSelectElement).value as CustomParamType,
            })}
        >
          <option value="string">string</option>
          <option value="number">number</option>
          <option value="boolean">boolean</option>
          <option value="json">json</option>
        </select>
        <span class="delete-btn-wrap">
          <ActionIconButton
            icon={Trash2}
            size={ICON_SIZE.md}
            title="Remove parameter"
            onclick={() => removeEntry(entry.id)}
          />
        </span>
      </div>
      <div class="value-row">
        {#if entry.type === "boolean"}
          <select
            class="value-input"
            value={entry.raw}
            onchange={(e) =>
              updateEntry(entry.id, {
                raw: (e.target as HTMLSelectElement).value,
              })}
          >
            <option value="true">true</option>
            <option value="false">false</option>
          </select>
        {:else if entry.type === "number"}
          <input
            class="value-input"
            type="number"
            step="any"
            value={entry.raw}
            oninput={(e) =>
              updateEntry(entry.id, {
                raw: (e.target as HTMLInputElement).value,
              })}
          />
        {:else if entry.type === "json"}
          <textarea
            class="value-input json-area"
            rows="3"
            value={entry.raw}
            oninput={(e) =>
              updateEntry(entry.id, {
                raw: (e.target as HTMLTextAreaElement).value,
              })}
          ></textarea>
        {:else}
          <input
            class="value-input"
            type="text"
            value={entry.raw}
            oninput={(e) =>
              updateEntry(entry.id, {
                raw: (e.target as HTMLInputElement).value,
              })}
          />
        {/if}
      </div>
      {#if err}
        <div class="error">{err}</div>
      {/if}
    </div>
  {/each}

  <button class="add-btn" onclick={addEntry}>
    <Plus size={ICON_SIZE.md} />
    <span>Add parameter</span>
  </button>
</div>

<style>
  .custom-params {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .entry {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4);
    background: var(--surface-sunken);
    border: 1px solid var(--border-hard);
    border-radius: 5px;
  }

  .entry.has-error {
    border-color: var(--danger-border);
  }

  .entry-row {
    display: flex;
    gap: var(--space-3);
    align-items: center;
  }

  .key-input {
    flex: 1;
    min-width: 0;
  }

  .type-select {
    width: 100px;
  }

  input,
  select,
  textarea {
    padding: 5px var(--space-4);
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

  .value-input {
    width: 100%;
  }

  .json-area {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    resize: vertical;
  }

  .delete-btn-wrap :global(.action-icon-btn:hover:not(:disabled)) {
    color: var(--danger);
    background: var(--danger-bg-soft);
  }

  .add-btn {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 5px var(--space-5);
    background: var(--surface-elevated);
    border: 1px solid var(--border-hard-2);
    border-radius: 5px;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-md);
    cursor: pointer;
  }

  .add-btn:hover {
    background: var(--surface-elevated);
  }

  .error {
    font-size: var(--font-size-sm);
    color: var(--danger);
  }
</style>
