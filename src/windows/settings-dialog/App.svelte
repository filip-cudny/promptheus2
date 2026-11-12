<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { Sparkles } from "lucide-svelte";
  import Button from "$lib/components/shared/ui/Button.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { syncTheme } from "$lib/stores/theme.svelte";
  import SettingsSidebar from "$lib/components/features/settings/SettingsSidebar.svelte";
  import { getSettingsNavigation } from "$lib/stores/settingsNavigation.svelte";
  import SettingsContent from "$lib/components/features/settings/SettingsContent.svelte";
  import MissingEnvList from "$lib/components/features/settings/MissingEnvList.svelte";
  import AgentPanel from "$lib/components/features/settings/agent/AgentPanel.svelte";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";
  import SettingsErrorToast from "$lib/components/features/settings/SettingsErrorToast.svelte";
  import {
    CONTENT_MIN_WIDTH,
    agentPanelLayout,
    clampPanelWidth,
  } from "$lib/utils/agentPanelLayout";

  const store = getSettingsStore();

  const navigation = getSettingsNavigation();
  const agent = getSettingsAgent();

  let bodyWidth = $state(0);
  const panelLayout = $derived(agentPanelLayout(bodyWidth, agent.panelWidth));

  type AgentPanelWindow = {
    __settingsAgentPanel?: unknown;
    __onSettingsAgentPanel?: (value: unknown) => void;
  };

  onMount(async () => {
    const agentWindow = window as unknown as AgentPanelWindow;
    agentWindow.__onSettingsAgentPanel = (value) => agent.applyInitialPanel(value);
    agent.applyInitialPanel(agentWindow.__settingsAgentPanel);
    const initial = (window as unknown as { __settingsInitialSection?: string })
      .__settingsInitialSection;
    if (initial === "models") navigation.activeSection = "models";
    await store.init();
  });

  $effect(() => {
    if (store.settings) syncTheme(store.settings.theme);
  });

  onDestroy(() => {
    delete (window as unknown as AgentPanelWindow).__onSettingsAgentPanel;
    store.destroy();
  });
</script>

<div class="dialog-shell">
  <MissingEnvList />
  <div
    class="dialog-body"
    bind:clientWidth={bodyWidth}
    style:--content-min-width="{CONTENT_MIN_WIDTH}px"
  >
    <SettingsSidebar />
    <SettingsContent />
    {#if !agent.panelOpen}
      <div class="agent-button">
        <Button icon={Sparkles} onclick={() => agent.openPanel("reopen")}>Show agent</Button>
      </div>
    {/if}
    {#if agent.panelOpen}
      <AgentPanel
        docked={panelLayout.docked}
        width={panelLayout.width}
        onresize={(width) => agent.resizePanel(clampPanelWidth(bodyWidth, width))}
        onresizeend={agent.savePanelWidth}
      />
    {/if}
  </div>
  <SettingsErrorToast />
</div>

<style>
  .dialog-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--surface-base);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--font-size-base);
    overflow: hidden;
  }

  .dialog-body {
    display: flex;
    flex: 1;
    min-height: 0;
    position: relative;
  }

  :global([data-settings-content] ::-webkit-scrollbar) {
    width: 10px;
  }

  :global([data-settings-content] ::-webkit-scrollbar-thumb:vertical) {
    border-right: 4px solid transparent;
    background-clip: padding-box;
  }

  :global([data-settings-content] ::-webkit-scrollbar-thumb:vertical:hover) {
    background-color: var(--border-strong);
  }

  .dialog-shell:has(:global(.resize-handle.dragging))
    :global([data-settings-content] ::-webkit-scrollbar-thumb) {
    background-color: transparent;
  }

  .dialog-shell:has(:global(.resize-handle.dragging)),
  .dialog-shell:has(:global(.resize-handle.dragging)) :global(*) {
    cursor: col-resize !important;
  }

  .agent-button {
    position: absolute;
    top: 10px;
    right: 12px;
    z-index: 5;
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }
</style>
