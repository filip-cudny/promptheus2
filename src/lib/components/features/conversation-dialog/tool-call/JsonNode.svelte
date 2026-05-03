<script lang="ts">
  import { ChevronRight, ChevronDown } from "lucide-svelte";
  import JsonNode from "./JsonNode.svelte";

  let {
    value,
    keyLabel,
    depth = 0,
  }: {
    value: unknown;
    keyLabel?: string;
    depth?: number;
  } = $props();

  const MAX_DEPTH = 10;

  let isObject = $derived(typeof value === "object" && value !== null && !Array.isArray(value));
  let isArray = $derived(Array.isArray(value));
  let itemCount = $derived(
    isArray
      ? (value as unknown[]).length
      : isObject
        ? Object.keys(value as Record<string, unknown>).length
        : 0,
  );
  let isCollapsible = $derived((isObject || isArray) && itemCount > 0);
  let entries = $derived(
    isArray
      ? (value as unknown[]).map((v, i) => [String(i), v] as [string, unknown])
      : isObject
        ? Object.entries(value as Record<string, unknown>)
        : [],
  );

  let defaultExpanded = $derived(depth < 2 && itemCount > 0);
  let expanded = $state<boolean | undefined>(undefined);
  let isExpanded = $derived(expanded ?? defaultExpanded);

  let bracketOpen = $derived(isArray ? "[" : "{");
  let bracketClose = $derived(isArray ? "]" : "}");
</script>

{#if depth > MAX_DEPTH}
  <span class="json-line">
    {#if keyLabel}<span class="json-key">{keyLabel}: </span>{/if}
    <span class="json-ellipsis">...</span>
  </span>
{:else if value === null}
  <span class="json-line">
    {#if keyLabel}<span class="json-key">{keyLabel}: </span>{/if}
    <span class="json-null">null</span>
  </span>
{:else if typeof value === "boolean"}
  <span class="json-line">
    {#if keyLabel}<span class="json-key">{keyLabel}: </span>{/if}
    <span class="json-bool">{String(value)}</span>
  </span>
{:else if typeof value === "number"}
  <span class="json-line">
    {#if keyLabel}<span class="json-key">{keyLabel}: </span>{/if}
    <span class="json-number">{value}</span>
  </span>
{:else if typeof value === "string"}
  <span class="json-line">
    {#if keyLabel}<span class="json-key">{keyLabel}: </span>{/if}
    <span class="json-string">"{value}"</span>
  </span>
{:else if isCollapsible}
  <div class="json-node">
    <button class="json-toggle" onclick={() => (expanded = !isExpanded)}>
      {#if isExpanded}
        <ChevronDown size={10} />
      {:else}
        <ChevronRight size={10} />
      {/if}
      {#if keyLabel}<span class="json-key">{keyLabel}: </span>{/if}
      <span class="json-bracket">{bracketOpen}</span>
      {#if !isExpanded}
        <span class="json-collapsed-hint">{itemCount} items</span>
        <span class="json-bracket">{bracketClose}</span>
      {/if}
    </button>
    {#if isExpanded}
      <div class="json-children">
        {#each entries as [key, val]}
          <div class="json-indent">
            <JsonNode value={val} keyLabel={isArray ? undefined : key} depth={depth + 1} />
          </div>
        {/each}
      </div>
      <span class="json-bracket">{bracketClose}</span>
    {/if}
  </div>
{:else if (isObject || isArray) && itemCount === 0}
  <span class="json-line">
    {#if keyLabel}<span class="json-key">{keyLabel}: </span>{/if}
    <span class="json-bracket">{isArray ? "[]" : "{}"}</span>
  </span>
{/if}

<style>
  .json-node {
    display: flex;
    flex-direction: column;
  }

  .json-toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    background: none;
    border: none;
    padding: var(--space-0);
    margin: var(--space-0);
    color: inherit;
    font: inherit;
    cursor: pointer;
  }

  .json-toggle:hover {
    background: var(--surface-overlay-faint);
    border-radius: 2px;
  }

  .json-children {
    display: flex;
    flex-direction: column;
  }

  .json-indent {
    padding-left: var(--space-8);
  }

  .json-line {
    display: block;
  }

  .json-key {
    color: rgba(200, 180, 230, 0.8);
  }

  .json-string {
    color: var(--success);
    word-break: break-word;
  }

  .json-number {
    color: rgba(130, 180, 230, 0.9);
  }

  .json-bool {
    color: var(--warning);
  }

  .json-null {
    color: var(--text-disabled);
    font-style: italic;
  }

  .json-bracket {
    color: var(--text-disabled);
  }

  .json-ellipsis {
    color: var(--text-faint);
  }

  .json-collapsed-hint {
    color: var(--text-faint);
    font-size: var(--font-size-xs);
    margin: var(--space-0) var(--space-2);
  }
</style>
