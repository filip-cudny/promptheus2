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
    gap: 4px;
  }

  .xml-node.nested {
    padding-left: 12px;
    border-left: 1px solid rgba(255, 255, 255, 0.06);
  }

  .xml-tag-header {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
  }

  .xml-tag-name {
    font-size: 11px;
    font-weight: 600;
    color: rgba(200, 180, 230, 0.8);
  }

  .xml-attrs {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .xml-attr {
    display: inline-flex;
    gap: 3px;
    font-size: 10px;
  }

  .xml-attr-key {
    color: rgba(255, 255, 255, 0.3);
  }

  .xml-attr-value {
    color: rgba(255, 255, 255, 0.55);
  }

  .xml-text {
    font-size: 11px;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.55);
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
  }

  .xml-children {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
</style>
