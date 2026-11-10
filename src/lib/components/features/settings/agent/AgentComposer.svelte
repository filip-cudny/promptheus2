<script lang="ts">
  import { onMount, tick, type Snippet } from "svelte";
  import { ArrowUp, Mic, Square } from "lucide-svelte";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";
  import { onSettingsAgentDictation } from "$lib/services/settingsAgent";

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

  const agent = getSettingsAgent();

  let input = $state<HTMLTextAreaElement>();

  $effect(() => {
    if (agent.focusRequest === 0) return;
    tick().then(() => input?.focus());
  });

  onMount(() => {
    const unlisten = onSettingsAgentDictation(async (dictated) => {
      agent.setDraft(agent.draft ? `${agent.draft} ${dictated}` : dictated);
      await tick();
      input?.focus();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  });

  function submit() {
    if (!agent.draft.trim() || running) return;
    onsend(agent.draft);
    agent.setDraft("");
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      submit();
    }
  }
</script>

<div class="composer">
  <div class="box">
    <textarea
      bind:this={input}
      bind:value={() => agent.draft, agent.setDraft}
      onkeydown={handleKeydown}
      rows="2"
      placeholder="Ask about or change a setting…"
    ></textarea>
    <div class="bar">
      {@render beforeSend?.()}
      <button class="icon" title="Dictate" disabled={running} onclick={() => agent.dictate()}>
        <Mic size={14} />
      </button>
      {#if running}
        <button class="send" title="Stop" onclick={oncancel}><Square size={14} /></button>
      {:else}
        <button class="send" title="Send" disabled={!agent.draft.trim()} onclick={submit}>
          <ArrowUp size={14} />
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .composer {
    padding: 10px 12px 12px;
    border-top: 1px solid var(--border-faint);
  }

  .box {
    max-width: 760px;
    margin: 0 auto;
    border: 1px solid var(--border-default);
    border-radius: 8px;
    background: var(--surface-sunken);
    padding: 8px 8px 6px 10px;
  }

  .box:focus-within {
    border-color: var(--accent-border);
  }

  textarea {
    display: block;
    width: 100%;
    resize: none;
    padding: 0;
    background: transparent;
    color: var(--text-primary);
    border: none;
    outline: none;
    font: inherit;
  }

  .bar {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    align-items: center;
    gap: 4px;
  }

  .icon,
  .send {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border: none;
    cursor: pointer;
  }

  .icon {
    background: transparent;
    color: var(--text-muted);
    border-radius: 4px;
  }

  .icon:hover:not(:disabled) {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }

  .send {
    background: var(--text-primary);
    color: var(--surface-base);
    border-radius: 50%;
  }

  .icon:disabled,
  .send:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
