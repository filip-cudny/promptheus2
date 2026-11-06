<script lang="ts">
  import { lineDiff } from "$lib/utils/lineDiff";
  import type { ProposalEffect, ProposalRowData } from "$lib/services/settingsAgent";

  const TESTABLE_OPS = [
    "add_model",
    "update_model",
    "create_skill",
    "update_skill",
    "add_mcp_server",
    "update_mcp_server",
  ];

  let {
    row,
    checked = $bindable(true),
    disabled = false,
    onshow,
    ontest,
  }: {
    row: ProposalRowData;
    checked?: boolean;
    disabled?: boolean;
    onshow: (effect: ProposalEffect) => void;
    ontest: () => void;
  } = $props();

  let testing = $state(false);

  const showable = $derived(row.effects.find((e) => canShow(e)) ?? null);

  function canShow(effect: ProposalEffect): boolean {
    if (effect.target.type === "setting") return true;
    return effect.target.path.startsWith("skills/") || effect.target.path.startsWith("prompts/");
  }

  function format(value: unknown): string {
    if (value === null || value === undefined) return "(none)";
    return typeof value === "string" ? value : JSON.stringify(value);
  }

  async function runTest() {
    testing = true;
    try {
      await ontest();
    } finally {
      testing = false;
    }
  }
</script>

<div class="row" class:stale={row.stale}>
  <label class="head">
    <input type="checkbox" bind:checked {disabled} />
    <span class="label">{row.label}</span>
  </label>
  {#if row.stale}<div class="stale-note">changed since proposed</div>{/if}
  {#each row.effects as effect, i (i)}
    <div class="effect">
      <code class="path">{effect.target.path}</code>
      {#if effect.target.type === "file"}
        <pre class="diff">{#each lineDiff(String(effect.before ?? ""), String(effect.after ?? "")) as line}<span class="line {line.kind}">{line.kind === "added" ? "+ " : line.kind === "removed" ? "- " : "  "}{line.text}
</span>{/each}</pre>
      {:else}
        <div class="values">
          <span class="before">{format(effect.before)}</span>
          <span class="arrow">→</span>
          <span class="after">{format(effect.after)}</span>
        </div>
      {/if}
    </div>
  {/each}
  {#if row.test}
    <div class="test" class:ok={row.test.ok}>
      {row.test.ok ? `OK · ${row.test.millis ?? 0} ms` : (row.test.error ?? "Test failed")}
    </div>
    {#if row.test.output}
      <details>
        <summary>Output</summary>
        <pre class="output">{row.test.output}</pre>
      </details>
    {/if}
  {/if}
  {#if row.sample_input}
    <details>
      <summary>Sample input</summary>
      <pre class="output">{row.sample_input}</pre>
    </details>
  {/if}
  <div class="links">
    {#if TESTABLE_OPS.includes(row.op)}
      <button type="button" disabled={testing} onclick={runTest}>{testing ? "Testing…" : "Test"}</button>
    {/if}
    {#if showable}
      <button type="button" onclick={() => onshow(showable)}>show</button>
    {/if}
  </div>
</div>

<style>
  .row {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) 0;
    border-top: 1px solid var(--border-faint);
  }

  .row.stale {
    opacity: 0.85;
  }

  .head {
    display: flex;
    gap: var(--space-3);
    align-items: center;
    font-weight: var(--font-weight-semibold);
  }

  .stale-note {
    color: var(--warning);
    font-size: var(--font-size-xs);
  }

  .effect {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--font-size-sm);
  }

  .path {
    color: var(--text-secondary);
    font-size: var(--font-size-xs);
    word-break: break-all;
  }

  .values {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    align-items: baseline;
  }

  .before {
    color: var(--danger);
    text-decoration: line-through;
    word-break: break-word;
  }

  .after {
    color: var(--success);
    word-break: break-word;
  }

  .diff,
  .output {
    margin: 0;
    max-height: 200px;
    overflow: auto;
    font-size: var(--font-size-xs);
    white-space: pre-wrap;
    background: var(--surface-overlay-faint);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-3);
  }

  .line {
    display: block;
  }

  .line.added {
    color: var(--success);
    background: var(--success-bg-soft);
  }

  .line.removed {
    color: var(--danger);
    background: var(--danger-bg-soft);
  }

  .test {
    font-size: var(--font-size-xs);
    color: var(--danger);
  }

  .test.ok {
    color: var(--success);
  }

  summary {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    cursor: pointer;
  }

  .links {
    display: flex;
    gap: var(--space-4);
  }

  .links button {
    background: transparent;
    border: none;
    padding: 0;
    color: var(--accent);
    font-size: var(--font-size-xs);
    cursor: pointer;
  }
</style>
