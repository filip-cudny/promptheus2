<script lang="ts">
  import PromptEditor from "./PromptEditor.svelte";
</script>

<div class="section-surface-prompts">
  <header class="section-head">
    <h2>Surface prompts</h2>
    <p class="hint">Per-feature prompts used outside the main chat surface.</p>
  </header>

  <PromptEditor
    kind="title_generation"
    description="Used to auto-name conversations after the first user message."
  />
  <PromptEditor
    kind="speech_to_text"
    description="Optional bias prompt for the speech-to-text model. Leave empty to disable."
  />
</div>

<style>
  .section-surface-prompts {
    padding: var(--space-8);
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
    overflow-y: auto;
  }

  .section-head {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  h2 {
    margin: 0;
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
  }

  .hint {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }
</style>
