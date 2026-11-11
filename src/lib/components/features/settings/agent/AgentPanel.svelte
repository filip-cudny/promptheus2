<script lang="ts">
  import { PanelRightClose, Sparkles, SquarePen } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { error as logError } from "@tauri-apps/plugin-log";
  import {
    agentModelId,
    agentModelName,
    agentModelState,
    agentReasoningEffort,
  } from "$lib/utils/agentModel";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";
  import { getCachedCapabilities, prefetchCapabilities } from "$lib/stores/capabilities.svelte";
  import { updateSurfaceModel, updateSurfaceReasoningEffort } from "$lib/services/settings";
  import ModelSelector from "$lib/components/shared/widgets/ModelSelector.svelte";
  import AgentMessages from "./AgentMessages.svelte";
  import AgentComposer from "./AgentComposer.svelte";

  let {
    docked,
    width,
    onresize,
    onresizeend,
  }: {
    docked: boolean;
    width: number;
    onresize: (width: number) => void;
    onresizeend: () => void;
  } = $props();

  const agent = getSettingsAgent();
  let panel: HTMLElement;
  let dragging = $state(false);
  const store = getSettingsStore();
  const modelState = $derived(
    store.settings ? agentModelState(store.settings.surfaces, store.models) : "ok",
  );
  const modelName = $derived.by(() => {
    if (!store.settings) return null;
    return modelState === "ok" ? agentModelName(store.settings.surfaces, store.models) : "No model";
  });
  const textModels = $derived(store.models.filter((m) => m.type === "text"));
  const effectiveId = $derived(store.settings ? agentModelId(store.settings.surfaces) : null);
  const effectiveEffort = $derived(
    store.settings ? agentReasoningEffort(store.settings.surfaces) : null,
  );
  const pickerVisible = $derived(textModels.some((m) => m.id === effectiveId));

  $effect(() => prefetchCapabilities(effectiveId));
  const capabilities = $derived(getCachedCapabilities(effectiveId));

  function selectModel(id: string) {
    updateSurfaceModel("settings_agent", id).catch((e) =>
      logError(`update_surface_model settings_agent failed: ${e}`),
    );
  }

  function selectEffort(effort: string | null) {
    updateSurfaceReasoningEffort("settings_agent", effort).catch((e) =>
      logError(`update reasoning_effort settings_agent failed: ${e}`),
    );
  }

  function startResize(e: PointerEvent) {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    dragging = true;
  }

  function resize(e: PointerEvent) {
    if (dragging) onresize(panel.getBoundingClientRect().right - e.clientX);
  }

  function endResize() {
    if (!dragging) return;
    dragging = false;
    onresizeend();
  }
</script>

<aside class="panel" class:docked style:width="{width}px" bind:this={panel}>
  <div
    class="resize-handle"
    class:dragging
    role="separator"
    aria-orientation="vertical"
    onpointerdown={startResize}
    onpointermove={resize}
    onpointerup={endResize}
    onpointercancel={endResize}
    onlostpointercapture={endResize}
  ></div>
  <header>
    <span class="title"><Sparkles size={14} />Settings agent{#if modelName}<span class="model" class:none={modelState !== "ok"} title={modelName}>{modelName}</span>{/if}</span>
    <button title="New conversation" disabled={agent.running} onclick={() => agent.reset()}>
      <SquarePen size={14} />
    </button>
    <button class="hide" title="Hide panel" onclick={() => agent.hidePanel("hide")}>
      <PanelRightClose size={ICON_SIZE.md} />Hide
    </button>
  </header>
  <AgentMessages {docked} />
  <AgentComposer
    running={agent.running}
    modelReady={modelState === "ok"}
    onsend={agent.send}
    oncancel={agent.cancel}
    beforeSend={modelPicker}
  />
</aside>

{#snippet modelPicker()}
  {#if pickerVisible}
    <ModelSelector
      models={textModels}
      selectedModelId={effectiveId}
      reasoningEffort={effectiveEffort}
      {capabilities}
      onModelSelect={selectModel}
      onReasoningSelect={selectEffort}
    />
  {/if}
{/snippet}

<style>
  .panel {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    max-width: 100%;
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    background: var(--surface-base);
    border-left: 1px solid var(--border-default);
    box-shadow: var(--shadow-lg, -4px 0 16px rgba(0, 0, 0, 0.3));
    z-index: 10;
  }

  .panel.docked {
    position: relative;
    max-width: calc(100% - var(--content-min-width));
    box-shadow: none;
    z-index: auto;
  }

  .resize-handle {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 6px;
    cursor: col-resize;
    z-index: 1;
    touch-action: none;
  }

  .resize-handle::after {
    content: "";
    position: absolute;
    left: 1px;
    top: 50%;
    width: 4px;
    height: 28px;
    margin-top: -14px;
    border-radius: 2px;
    background: var(--border-strong);
  }

  .resize-handle:hover::after,
  .resize-handle.dragging::after {
    background: var(--accent);
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
    min-width: 0;
    font-weight: var(--font-weight-semibold);
    color: var(--text-secondary);
  }

  .model {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-regular);
    color: var(--text-muted);
  }

  .model.none {
    color: var(--warning);
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

  header button.hide {
    width: auto;
    gap: 6px;
    padding: 0 8px;
    border: 1px solid var(--border-default);
    font: inherit;
    font-size: var(--font-size-xs);
  }

  header button:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
