<script lang="ts">
  import { parseToolResult } from "$lib/utils/toolResultParser";
  import { isXmlLike } from "$lib/utils/xmlParser";
  import type { HintStatus } from "$lib/types/toolResult";
  import MarkdownRenderer from "$lib/components/ui/MarkdownRenderer.svelte";
  import SearchResultsRenderer from "./SearchResultsRenderer.svelte";
  import JsonNode from "./JsonNode.svelte";

  let {
    result,
    isPartial = false,
  }: {
    result: string | null;
    isPartial?: boolean;
  } = $props();

  let parsed = $derived(isPartial ? null : parseToolResult(result));

  const STATUS_COLORS: Record<HintStatus, { bg: string; fg: string }> = {
    success: { bg: "rgba(92, 184, 92, 0.15)", fg: "#5cb85c" },
    error: { bg: "rgba(238, 85, 85, 0.15)", fg: "#e55" },
    empty: { bg: "rgba(212, 168, 67, 0.15)", fg: "#d4a843" },
    partial: { bg: "rgba(91, 141, 217, 0.15)", fg: "#5b8dd9" },
  };

  const MARKDOWN_PATTERN = /(?:^#{1,6}\s|^\s*[-*]\s|\*\*|```|\[.+\]\()/m;

  function isMarkdownLike(text: string): boolean {
    return MARKDOWN_PATTERN.test(text);
  }

  function formatMetadataValue(value: unknown): string {
    if (typeof value === "number") {
      if (value > 1000 && Number.isInteger(value)) return `${(value / 1000).toFixed(1)}s`;
      return String(value);
    }
    return String(value);
  }

  function extractEnvelopeData(data: unknown): {
    formattedContext: string | null;
    metadata: Record<string, unknown> | null;
    otherData: Record<string, unknown> | null;
  } {
    if (typeof data !== "object" || data === null) {
      return { formattedContext: null, metadata: null, otherData: null };
    }
    const obj = data as Record<string, unknown>;
    const formattedContext =
      typeof obj.formattedContext === "string" ? obj.formattedContext : null;
    const metadata =
      typeof obj.metadata === "object" && obj.metadata !== null
        ? (obj.metadata as Record<string, unknown>)
        : null;

    const rest: Record<string, unknown> = {};
    let hasRest = false;
    for (const [key, val] of Object.entries(obj)) {
      if (key !== "formattedContext" && key !== "metadata") {
        rest[key] = val;
        hasRest = true;
      }
    }

    return {
      formattedContext,
      metadata,
      otherData: hasRest ? rest : null,
    };
  }
</script>

{#if isPartial && result}
  <span class="partial-text">{result}</span>
{:else if parsed}
  {#if parsed.kind === "envelope"}
    {@const colors = STATUS_COLORS[parsed.hint.status]}
    {@const envelope = extractEnvelopeData(parsed.data)}
    <div class="envelope-result">
      <div class="envelope-header">
        <span class="status-pill" style:background={colors.bg} style:color={colors.fg}>
          {parsed.hint.status}
        </span>
        <span class="hint-summary">{parsed.hint.summary}</span>
      </div>

      {#if envelope.formattedContext}
        {#if isXmlLike(envelope.formattedContext)}
          <SearchResultsRenderer xml={envelope.formattedContext} />
        {:else}
          <pre class="formatted-context">{envelope.formattedContext}</pre>
        {/if}
      {/if}

      {#if envelope.metadata}
        <div class="metadata-grid">
          {#each Object.entries(envelope.metadata) as [key, val]}
            <span class="meta-key">{key}</span>
            <span class="meta-value">{formatMetadataValue(val)}</span>
          {/each}
        </div>
      {/if}

      {#if envelope.otherData}
        <div class="envelope-data">
          <JsonNode value={envelope.otherData} />
        </div>
      {/if}

      {#if parsed.hint.diagnostics}
        <div class="diagnostics">
          <span class="section-label">Diagnostics</span>
          <div class="metadata-grid">
            {#each Object.entries(parsed.hint.diagnostics) as [key, val]}
              <span class="meta-key">{key}</span>
              <span class="meta-value">{val}</span>
            {/each}
          </div>
        </div>
      {/if}

      {#if parsed.hint.nextActions.length > 0}
        <div class="next-actions">
          <span class="section-label">Suggested</span>
          {#each parsed.hint.nextActions as action}
            <div class="action-item">
              <span class="action-tool">{action.tool}</span>
              <span class="action-why">{action.why}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else if parsed.kind === "json"}
    <div class="json-viewer">
      <JsonNode value={parsed.value} />
    </div>
  {:else if parsed.kind === "text"}
    {#if isMarkdownLike(parsed.text)}
      <MarkdownRenderer content={parsed.text} isStreaming={false} />
    {:else}
      <pre class="plain-text">{parsed.text}</pre>
    {/if}
  {/if}
{/if}

<style>
  .partial-text {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.4);
  }

  .envelope-result {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .envelope-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    font-size: 10px;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: 3px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    flex-shrink: 0;
  }

  .hint-summary {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.7);
  }

  .formatted-context {
    font-size: 11px;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.6);
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
    max-height: 250px;
    overflow-y: auto;
  }

  .metadata-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 2px 10px;
    font-size: 11px;
  }

  .meta-key {
    color: rgba(255, 255, 255, 0.35);
  }

  .meta-value {
    color: rgba(255, 255, 255, 0.55);
    font-variant-numeric: tabular-nums;
  }

  .section-label {
    display: block;
    font-size: 10px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.3);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 4px;
  }

  .diagnostics,
  .next-actions {
    padding-top: 4px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  .action-item {
    display: flex;
    gap: 6px;
    font-size: 11px;
    padding: 1px 0;
  }

  .action-tool {
    color: rgba(91, 141, 217, 0.8);
    font-weight: 500;
    flex-shrink: 0;
  }

  .action-why {
    color: rgba(255, 255, 255, 0.4);
  }

  .envelope-data {
    font-family: "Fira Code", "Cascadia Code", monospace;
    font-size: 11px;
    line-height: 1.4;
  }

  .json-viewer {
    font-family: "Fira Code", "Cascadia Code", monospace;
    font-size: 11px;
    line-height: 1.4;
    max-height: 300px;
    overflow-y: auto;
  }

  .plain-text {
    font-size: 12px;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.65);
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
  }
</style>
