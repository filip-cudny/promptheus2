<script lang="ts">
  import type { Snippet } from "svelte";
  import { ArrowUp, Square } from "lucide-svelte";

  let {
    running,
    onsend,
    oncancel,
    beforeSend,
  }: {
    running: boolean;
    onsend: (text: string) => void;
    oncancel: () => void;
    beforeSend?: Snippet;
  } = $props();

  let text = $state("");

  function submit() {
    if (!text.trim() || running) return;
    onsend(text);
    text = "";
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      submit();
    }
  }
</script>

<div class="composer">
  <textarea
    bind:value={text}
    onkeydown={handleKeydown}
    rows="2"
    placeholder="Ask about settings"
  ></textarea>
  <div class="actions">
    {@render beforeSend?.()}
    {#if running}
      <button class="action" title="Stop" onclick={oncancel}><Square size={14} /></button>
    {:else}
      <button class="action" title="Send" disabled={!text.trim()} onclick={submit}>
        <ArrowUp size={14} />
      </button>
    {/if}
  </div>
</div>

<style>
  .composer {
    display: flex;
    align-items: flex-end;
    gap: var(--space-4);
    padding: var(--space-6);
    border-top: 1px solid var(--border-faint);
  }

  textarea {
    flex: 1;
    resize: none;
    padding: var(--space-4);
    background: var(--surface-sunken);
    color: var(--text-primary);
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-md);
    font: inherit;
  }

  .actions {
    display: flex;
    gap: var(--space-2);
  }

  .action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: var(--accent-bg-soft);
    color: var(--text-primary);
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
  }

  .action:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
