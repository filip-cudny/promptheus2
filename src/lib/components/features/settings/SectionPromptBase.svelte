<script lang="ts">
  import PromptEditor from "./PromptEditor.svelte";
</script>

<div class="section-prompt-base">
  <header class="section-head">
    <h2>Prompt Base</h2>
    <p class="hint">Templates that build the system message of every chat conversation.</p>
  </header>

  <PromptEditor
    kind="system"
    description="Top-level instructions sent as the assistant's system role."
  />
  <PromptEditor
    kind="about_me"
    description="Long-form context about you. Appended to every conversation's system message."
  />
  <PromptEditor
    kind="environment"
    description="Per-conversation snapshot built when chat starts. Use placeholders for dynamic values."
  />
  <PromptEditor
    kind="input_format"
    description="Tags and conventions the model should expect in user messages (context, pasted text, images, skills)."
  />
</div>

<style>
  .section-prompt-base {
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
