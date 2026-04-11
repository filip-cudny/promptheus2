<script lang="ts">
  import { ChevronRight, ChevronDown } from "lucide-svelte";

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
  let isCollapsible = $derived((isObject || isArray) && itemCount > 0);
  let itemCount = $derived(
    isArray
      ? (value as unknown[]).length
      : isObject
        ? Object.keys(value as Record<string, unknown>).length
        : 0,
  );
  let entries = $derived(
    isArray
      ? (value as unknown[]).map((v, i) => [String(i), v] as [string, unknown])
      : isObject
        ? Object.entries(value as Record<string, unknown>)
        : [],
  );

  let expanded = $state(depth < 2 && itemCount > 0);

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
    <button class="json-toggle" onclick={() => (expanded = !expanded)}>
      {#if expanded}
        <ChevronDown size={10} />
      {:else}
        <ChevronRight size={10} />
      {/if}
      {#if keyLabel}<span class="json-key">{keyLabel}: </span>{/if}
      <span class="json-bracket">{bracketOpen}</span>
      {#if !expanded}
        <span class="json-collapsed-hint">{itemCount} items</span>
        <span class="json-bracket">{bracketClose}</span>
      {/if}
    </button>
    {#if expanded}
      <div class="json-children">
        {#each entries as [key, val]}
          <div class="json-indent">
            <svelte:self value={val} keyLabel={isArray ? undefined : key} depth={depth + 1} />
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
    gap: 2px;
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }

  .json-toggle:hover {
    background: rgba(255, 255, 255, 0.04);
    border-radius: 2px;
  }

  .json-children {
    display: flex;
    flex-direction: column;
  }

  .json-indent {
    padding-left: 16px;
  }

  .json-line {
    display: block;
  }

  .json-key {
    color: rgba(200, 180, 230, 0.8);
  }

  .json-string {
    color: rgba(140, 200, 140, 0.8);
    word-break: break-word;
  }

  .json-number {
    color: rgba(130, 180, 230, 0.9);
  }

  .json-bool {
    color: rgba(212, 168, 67, 0.9);
  }

  .json-null {
    color: rgba(255, 255, 255, 0.3);
    font-style: italic;
  }

  .json-bracket {
    color: rgba(255, 255, 255, 0.35);
  }

  .json-ellipsis {
    color: rgba(255, 255, 255, 0.25);
  }

  .json-collapsed-hint {
    color: rgba(255, 255, 255, 0.25);
    font-size: 10px;
    margin: 0 4px;
  }
</style>
