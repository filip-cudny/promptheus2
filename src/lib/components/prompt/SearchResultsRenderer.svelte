<script lang="ts">
  import { parseXml, type XmlNode } from "$lib/utils/xmlParser";
  import XmlNodeRenderer from "./XmlNodeRenderer.svelte";
  import { ExternalLink } from "lucide-svelte";
  import { openUrl as tauriOpenUrl } from "@tauri-apps/plugin-opener";

  let {
    xml,
  }: {
    xml: string;
  } = $props();

  let root = $derived(parseXml(xml));
  let isWebSearch = $derived(root?.tag === "web_search");
  let results = $derived(
    isWebSearch ? root!.children.filter((c) => c.tag === "result") : [],
  );

  function getChildText(node: XmlNode, tag: string): string | null {
    return node.children.find((c) => c.tag === tag)?.text ?? null;
  }

  function getContentChunks(node: XmlNode): string[] {
    const raw = getChildText(node, "content");
    if (!raw) return [];
    return raw
      .split(/\n\s*---\s*\n/)
      .map((c) => c.trim())
      .filter(Boolean);
  }

  let expandedSet = $state(new Set<number>());
  const CONTENT_PREVIEW_LENGTH = 200;

  function toggleExpand(index: number) {
    const next = new Set(expandedSet);
    if (next.has(index)) {
      next.delete(index);
    } else {
      next.add(index);
    }
    expandedSet = next;
  }

  function openUrl(url: string) {
    tauriOpenUrl(url);
  }
</script>

{#if isWebSearch && results.length > 0}
  <div class="search-results">
    {#each results as result, i}
      {@const title = getChildText(result, "title")}
      {@const url = result.attributes.url}
      {@const domain = result.attributes.domain}
      {@const date = result.attributes.date}
      {@const relevance = result.attributes.relevance}
      {@const chunks = getContentChunks(result)}
      {@const fullContent = chunks.join("\n\n")}
      {@const isLong = fullContent.length > CONTENT_PREVIEW_LENGTH}
      {@const isExpanded = expandedSet.has(i)}

      <div class="result-card">
        <div class="result-header">
          {#if url}
            <button class="result-title-link" onclick={() => openUrl(url)}>
              <span class="result-title">{title ?? url}</span>
              <ExternalLink size={11} />
            </button>
          {:else if title}
            <span class="result-title">{title}</span>
          {/if}
          {#if relevance}
            <span class="relevance-pill" title="Relevance score">{relevance}</span>
          {/if}
        </div>

        {#if domain || date}
          <div class="result-meta">
            {#if domain}<span class="result-domain">{domain}</span>{/if}
            {#if domain && date}<span class="meta-sep">&middot;</span>{/if}
            {#if date}<span class="result-date">{date}</span>{/if}
          </div>
        {/if}

        {#if fullContent}
          <pre class="result-content">{isLong && !isExpanded
              ? fullContent.slice(0, CONTENT_PREVIEW_LENGTH) + "…"
              : fullContent}</pre>
          {#if isLong}
            <button class="expand-toggle" onclick={() => toggleExpand(i)}>
              {isExpanded ? "Zwiń" : "Rozwiń"}
            </button>
          {/if}
        {/if}

        {#each result.children.filter((c) => c.tag !== "title" && c.tag !== "content") as unknownChild}
          <div class="unknown-child">
            <XmlNodeRenderer node={unknownChild} />
          </div>
        {/each}
      </div>
    {/each}
  </div>
{:else if root}
  <XmlNodeRenderer node={root} />
{:else}
  <pre class="fallback-text">{xml}</pre>
{/if}

<style>
  .search-results {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .result-card {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 6px 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .result-card:last-child {
    border-bottom: none;
  }

  .result-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .result-title-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    color: rgba(130, 180, 230, 0.9);
    font: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    text-align: left;
  }

  .result-title-link:hover {
    color: rgba(150, 200, 255, 1);
  }

  .result-title-link :global(svg) {
    flex-shrink: 0;
    opacity: 0.5;
  }

  .result-title {
    font-size: 12px;
    font-weight: 500;
  }

  .relevance-pill {
    font-size: 9px;
    font-weight: 700;
    color: rgba(255, 255, 255, 0.4);
    background: rgba(255, 255, 255, 0.06);
    padding: 1px 5px;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .result-meta {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    color: rgba(255, 255, 255, 0.35);
  }

  .meta-sep {
    color: rgba(255, 255, 255, 0.2);
  }

  .result-content {
    font-size: 11px;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.5);
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
  }

  .expand-toggle {
    background: none;
    border: none;
    padding: 0;
    color: rgba(130, 180, 230, 0.6);
    font-size: 10px;
    cursor: pointer;
    align-self: flex-start;
  }

  .expand-toggle:hover {
    color: rgba(130, 180, 230, 0.9);
  }

  .unknown-child {
    padding-top: 4px;
    border-top: 1px solid rgba(255, 255, 255, 0.04);
  }

  .fallback-text {
    font-size: 11px;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.6);
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
  }
</style>
