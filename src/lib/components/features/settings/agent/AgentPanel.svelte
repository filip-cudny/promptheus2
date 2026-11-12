<script lang="ts">
  import { Globe, PanelRightClose, SquarePen } from "lucide-svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import { error as logError } from "@tauri-apps/plugin-log";
  import {
    agentModelHasBuiltinSearch,
    agentModelId,
    agentModelName,
    agentModelState,
    agentReasoningEffort,
  } from "$lib/utils/agentModel";
  import { getSettingsAgent } from "$lib/stores/settingsAgent.svelte";
  import { getCachedCapabilities, prefetchCapabilities } from "$lib/stores/capabilities.svelte";
  import { setSurfaceTool, updateSurfaceModel, updateSurfaceReasoningEffort } from "$lib/services/settings";
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
  const builtinSearchVisible = $derived(
    store.settings ? agentModelHasBuiltinSearch(store.settings.surfaces, store.models) : false,
  );
  const webSearchOn = $derived(
    store.settings?.surfaces.settings_agent.generation.enabled_tools.includes("web_search") ?? false,
  );

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

  function toggleWebSearch() {
    setSurfaceTool("settings_agent", "web_search", !webSearchOn).catch((e) =>
      logError(`set_surface_tool settings_agent failed: ${e}`),
    );
  }

  function startResize(e: PointerEvent) {
    e.preventDefault();
    (document.activeElement as HTMLElement | null)?.blur();
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
    <button class="icon-btn" title="Hide panel" onclick={() => agent.hidePanel("hide")}>
      <PanelRightClose size={16} />
    </button>
    <span class="title">Settings agent{#if modelName}<span class="model" class:none={modelState !== "ok"} title={modelName}>{modelName}</span>{/if}</span>
    <button class="icon-btn" title="New conversation" disabled={agent.running} onclick={() => agent.reset()}>
      <SquarePen size={16} />
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
  {#if builtinSearchVisible}
    <button
      class="web-search"
      class:on={webSearchOn}
      title="Web search"
      aria-pressed={webSearchOn}
      onclick={toggleWebSearch}
    >
      <Globe size={14} />
    </button>
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
    left: -5px;
    width: 9px;
    cursor: col-resize;
    z-index: 1;
    touch-action: none;
  }

  .resize-handle::before {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 3px;
    width: 2px;
    background-color: transparent;
    transition: background-color 0.12s ease 0.15s;
  }

  .resize-handle:hover::before,
  .resize-handle.dragging::before {
    background-color: var(--accent);
  }

  .resize-handle.dragging::before {
    transition: none;
  }

  header {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    height: 40px;
    gap: 4px;
    padding: 0 6px;
    border-bottom: 1px solid var(--border-faint);
  }

  .title {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex: 1;
    min-width: 0;
    padding-left: 2px;
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
  }

  .model {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-regular);
    color: var(--text-muted);
  }

  .model::before {
    content: "·";
    margin-right: 8px;
    color: var(--text-muted);
  }

  .model.none {
    color: var(--warning);
  }

  .icon-btn {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .icon-btn:hover:not(:disabled) {
    background: var(--surface-raised);
    color: var(--text-primary);
  }

  .icon-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .web-search {
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

  .web-search:hover {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }

  .web-search.on,
  .web-search.on:hover {
    background: var(--accent-bg-soft);
    color: var(--accent);
  }
</style>
