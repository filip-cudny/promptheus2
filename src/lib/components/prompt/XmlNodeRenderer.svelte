<script lang="ts">
  import type { XmlNode } from "$lib/utils/xmlParser";
  import XmlNodeRenderer from "./XmlNodeRenderer.svelte";

  let {
    node,
    depth = 0,
  }: {
    node: XmlNode;
    depth?: number;
  } = $props();

  let attrEntries = $derived(Object.entries(node.attributes));
  let hasAttributes = $derived(attrEntries.length > 0);
</script>

<div class="xml-node" class:nested={depth > 0}>
  <div class="xml-tag-header">
    <span class="xml-tag-name">{node.tag}</span>
    {#if hasAttributes}
      <div class="xml-attrs">
        {#each attrEntries as [key, val]}
          <span class="xml-attr">
            <span class="xml-attr-key">{key}</span>
            <span class="xml-attr-value">{val}</span>
          </span>
        {/each}
      </div>
    {/if}
  </div>

  {#if node.text}
    <pre class="xml-text">{node.text}</pre>
  {/if}

  {#if node.children.length > 0}
    <div class="xml-children">
      {#each node.children as child}
        <XmlNodeRenderer node={child} depth={depth + 1} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .xml-node {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .xml-node.nested {
    padding-left: var(--space-6);
    border-left: 1px solid var(--border-faint);
  }

  .xml-tag-header {
    display: flex;
    align-items: baseline;
    gap: var(--space-4);
    flex-wrap: wrap;
  }

  .xml-tag-name {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    color: rgba(200, 180, 230, 0.8);
  }

  .xml-attrs {
    display: flex;
    gap: var(--space-4);
    flex-wrap: wrap;
  }

  .xml-attr {
    display: inline-flex;
    gap: 3px;
    font-size: var(--font-size-xs);
  }

  .xml-attr-key {
    color: var(--text-disabled);
  }

  .xml-attr-value {
    color: var(--text-muted);
  }

  .xml-text {
    font-size: var(--font-size-sm);
    line-height: var(--line-height-normal);
    color: var(--text-muted);
    white-space: pre-wrap;
    word-break: break-word;
    margin: var(--space-0);
  }

  .xml-children {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
</style>
