<script lang="ts">
  import { Trash2 } from "lucide-svelte";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";
  import AgentMessages from "./AgentMessages.svelte";
  import AgentComposer from "./AgentComposer.svelte";

  const agent = getSettingsAgent();
</script>

<aside class="panel" class:overlay={agent.panelMode === "overlay"}>
  <header>
    <span>Settings agent</span>
    <button title="New conversation" disabled={agent.running} onclick={() => agent.reset()}>
      <Trash2 size={14} />
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
    height: 100%;
    background: var(--surface-base);
    border-left: 1px solid var(--border-faint);
  }

  .panel.overlay {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    box-shadow: var(--shadow-lg, -4px 0 16px rgba(0, 0, 0, 0.3));
    z-index: 10;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-6);
    border-bottom: 1px solid var(--border-faint);
    font-weight: var(--font-weight-semibold);
    color: var(--text-secondary);
  }

  header button {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
  }
</style>
