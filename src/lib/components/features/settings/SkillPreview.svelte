<script lang="ts">
  import { ChevronRight, Play } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { previewSkillMessage } from "$lib/services/skills";

  let {
    body,
    skillName,
  }: {
    body: string;
    skillName: string;
  } = $props();

  let expanded = $state(false);
  let sampleInput = $state("Sample input text here.");
  let rendered = $state<string>("");
  let pending = $state(false);
  let error = $state<string | null>(null);

  async function refreshPreview() {
    pending = true;
    error = null;
    try {
      rendered = await previewSkillMessage(body, sampleInput, skillName);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      pending = false;
    }
  }

  $effect(() => {
    if (!expanded) return;
    body;
    sampleInput;
    skillName;
    void refreshPreview();
  });
</script>

<section class="card">
  <button class="toggle" class:open={expanded} onclick={() => (expanded = !expanded)}>
    <ChevronRight size={ICON_SIZE.sm} />
    <span>Preview composed message</span>
    <Play size={ICON_SIZE.sm} class="muted" />
  </button>

  {#if expanded}
    <div class="body">
      <label class="sample-row">
        <span>Sample input</span>
        <textarea
          rows="2"
          bind:value={sampleInput}
          spellcheck="false"
        ></textarea>
      </label>

      <div class="output-wrap">
        {#if pending}
          <div class="status">Rendering…</div>
        {:else if error}
          <div class="status err">{error}</div>
        {:else}
          <pre class="output">{rendered}</pre>
        {/if}
      </div>

      <p class="footnote">
        Placeholders like <code>{`{{date}}`}</code> are substituted with current values; Test
        run is intentionally out of scope — invoke the skill from the command palette to run it
        for real.
      </p>
    </div>
  {/if}
</section>

<style>
  .card {
    background: var(--surface-base);
    border: 1px solid var(--border-hard);
    border-radius: var(--radius-lg);
    padding: var(--space-4) var(--space-7);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-sm);
    cursor: pointer;
    align-self: flex-start;
  }

  .toggle :global(svg) {
    transition: transform var(--motion-fast) var(--ease-default);
  }

  .toggle.open :global(svg:first-child) {
    transform: rotate(90deg);
  }

  .toggle :global(.muted) {
    color: var(--text-faint);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding-bottom: var(--space-3);
  }

  .sample-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .sample-row span {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .sample-row textarea {
    padding: var(--space-3) var(--space-4);
    background: var(--surface-sunken);
    border: 1px solid var(--border-hard);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    resize: vertical;
  }

  .output-wrap {
    background: var(--surface-sunken);
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-md);
    padding: var(--space-4);
    max-height: 320px;
    overflow: auto;
  }

  .output {
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    line-height: 1.55;
    color: var(--text-primary);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .status {
    color: var(--text-muted);
    font-size: var(--font-size-sm);
  }

  .status.err {
    color: var(--danger);
  }

  .footnote {
    font-size: var(--font-size-xs);
    color: var(--text-faint);
    margin: 0;
  }

  .footnote code {
    font-family: var(--font-mono);
    padding: 0 4px;
    background: var(--surface-overlay-faint);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
  }
</style>
