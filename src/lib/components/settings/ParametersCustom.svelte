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
        <button
          class="delete-btn"
          title="Remove parameter"
          onclick={() => removeEntry(entry.id)}
        >
          <Trash2 size={ICON_SIZE.md} />
        </button>
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
    gap: 10px;
  }

  .entry {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px;
    background: #1a1a1a;
    border: 1px solid #2e2e2e;
    border-radius: 5px;
  }

  .entry.has-error {
    border-color: rgba(217, 115, 115, 0.5);
  }

  .entry-row {
    display: flex;
    gap: 6px;
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
    padding: 5px 8px;
    background: #1f1f1f;
    border: 1px solid #3a3a3a;
    border-radius: 4px;
    color: rgba(255, 255, 255, 0.92);
    font: inherit;
    font-size: 12px;
  }

  .value-input {
    width: 100%;
  }

  .json-area {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 11px;
    resize: vertical;
  }

  .delete-btn {
    background: transparent;
    border: 1px solid transparent;
    color: rgba(255, 255, 255, 0.45);
    padding: 4px;
    border-radius: 4px;
    cursor: pointer;
    display: inline-flex;
  }

  .delete-btn:hover {
    color: #d97373;
    background: rgba(217, 115, 115, 0.08);
  }

  .add-btn {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 5px 10px;
    background: #2a2a2a;
    border: 1px solid #3e3e3e;
    border-radius: 5px;
    color: rgba(255, 255, 255, 0.78);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .add-btn:hover {
    background: #333;
  }

  .error {
    font-size: 11px;
    color: #d97373;
  }
</style>
