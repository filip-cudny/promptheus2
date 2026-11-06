<script lang="ts">
  import { Sparkles, SquarePen, X } from "lucide-svelte";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";
  import AgentMessages from "./AgentMessages.svelte";
  import AgentComposer from "./AgentComposer.svelte";

  const agent = getSettingsAgent();
</script>

<aside class="panel">
  <header>
    <span class="title"><Sparkles size={14} />Settings agent</span>
    <button title="New conversation" disabled={agent.running} onclick={() => agent.reset()}>
      <SquarePen size={14} />
    </button>
    <button title="Close" onclick={() => agent.setPanelOpen(false)}>
      <X size={14} />
    </button>
  </header>
  <AgentMessages />
  <AgentComposer running={agent.running} onsend={agent.send} oncancel={agent.cancel} />
</aside>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    width: 380px;
    flex-shrink: 0;
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    background: var(--surface-base);
    border-left: 1px solid var(--border-default);
    box-shadow: var(--shadow-lg, -4px 0 16px rgba(0, 0, 0, 0.3));
    z-index: 10;
  }

  @container settings-body (min-width: 1340px) {
    .panel {
      position: static;
      width: auto;
      flex: 1 1 380px;
      min-width: 380px;
      box-shadow: none;
      z-index: auto;
    }
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: 10px 12px 10px 16px;
    border-bottom: 1px solid var(--border-faint);
  }

  .title {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    flex: 1;
    font-weight: var(--font-weight-semibold);
    color: var(--text-secondary);
  }

  .title :global(svg) {
    color: var(--text-muted);
  }

  header button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
  }

  header button:hover:not(:disabled) {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }

  header button:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
