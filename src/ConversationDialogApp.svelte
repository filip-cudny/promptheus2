<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { createConversationStore } from "$lib/stores/conversation.svelte";
  import { hasContext } from "$lib/services/context";
  import { getSettings } from "$lib/services/settings";
  import { getContextWindowSize } from "$lib/utils/contextWindow";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { PanelLeft, RefreshCw, SquarePen } from "lucide-svelte";
  import { providerIconSvg } from "$lib/icons/providerIcons";
  import { getSkillsStore } from "$lib/stores/skills.svelte";
  import ConversationArea from "$lib/components/prompt/ConversationArea.svelte";
  import InputArea from "$lib/components/prompt/InputArea.svelte";
  import TabSidebar from "$lib/components/prompt/TabSidebar.svelte";
  import {
    PROMPTHEUS_PROVIDER_ID,
    closePalette,
    openPalette,
    reloadActiveInHost,
  } from "$lib/services/shellToolbar";
  import { getWebviewProviders, type WebviewProvider } from "$lib/services/aiWebview";
  import { onSettingsChanged } from "$lib/services/events";

  interface DialogInitParams {
    skill_id: string;
    skill_name: string;
    skill_model: string | null;
    history_entry_id: string | null;
    last_interaction_only: boolean;
    initial_input: string | null;
    auto_send_input: boolean;
    new_chat: boolean;
  }

  type ProviderEntry = { kind: "provider"; id: string; name: string; url?: string };
  type ActionEntry = { kind: "action"; id: string; name: string };
  type PaletteEntry = ProviderEntry | ActionEntry;

  const ACTION_RELOAD_ID = "action:reload-active";

  const skillsStore = getSkillsStore();
  const HOST_LABEL = getCurrentWindow().label;
  const SELF_TARGET = getCurrentWebview().label;

  const store = createConversationStore("", "");

  import type { ModelConfig } from "$lib/types";

  let sidebarOpen = $state(false);
  let contextVisible = $state(false);
  let contextDisabled = $state(false);
  let contextInitialCollapsed = $state(false);
  let models = $state<ModelConfig[]>([]);
  let defaultModelId = $state<string | null>(null);

  let webviewProviders = $state<WebviewProvider[]>([]);
  let paletteOpen = $state(false);
  let paletteQuery = $state("");
  let paletteIndex = $state(0);
  let paletteInputEl: HTMLInputElement | undefined = $state();
  let paletteActiveId = $state<string>(PROMPTHEUS_PROVIDER_ID);

  let providers = $derived<ProviderEntry[]>([
    { kind: "provider", id: PROMPTHEUS_PROVIDER_ID, name: "Promptheus" },
    ...webviewProviders.map<ProviderEntry>((p) => ({ kind: "provider", id: p.id, name: p.name, url: p.url })),
  ]);

  let activeProviderName = $derived(
    providers.find((p) => p.id === paletteActiveId)?.name ?? "active provider",
  );

  let actions = $derived<ActionEntry[]>([
    { kind: "action", id: ACTION_RELOAD_ID, name: `Reload ${activeProviderName}` },
  ]);

  let entries = $derived<PaletteEntry[]>([...providers, ...actions]);

  let filtered = $derived.by<PaletteEntry[]>(() => {
    const q = paletteQuery.trim().toLowerCase();
    if (!q) return entries;
    return entries.filter((e) => e.name.toLowerCase().includes(q));
  });

  let contextWindowSize = $derived.by(() => {
    const activeModelId = store.modelId ?? defaultModelId;
    const activeModel = models.find((m) => m.id === activeModelId);
    if (!activeModel) return 0;
    return getContextWindowSize(activeModel.model, activeModel.context_window_size);
  });

  let unlistenRestore: UnlistenFn | undefined;
  let unlistenContextChanged: UnlistenFn | undefined;
  let unlistenVoiceInput: UnlistenFn | undefined;
  let unlistenOpenForSkill: UnlistenFn | undefined;
  let unlistenNewConversation: UnlistenFn | undefined;
  let unlistenPaletteOpened: UnlistenFn | undefined;
  let unlistenPaletteClosed: UnlistenFn | undefined;
  let unlistenActive: UnlistenFn | undefined;
  let unlistenSettingsChanged: UnlistenFn | undefined;

  async function refreshWebviewProviders() {
    try {
      webviewProviders = await getWebviewProviders();
    } catch (e) {
      console.error("getWebviewProviders failed", e);
    }
  }

  async function handleGlobalKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "r") {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (paletteOpen) {
        await dismissPalette(null);
      }
      await reloadActiveProvider();
      return;
    }

    if (paletteOpen) {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopImmediatePropagation();
        await dismissPalette(null);
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        e.stopImmediatePropagation();
        const entry = filtered[paletteIndex];
        if (entry) {
          await selectEntry(entry);
        }
        return;
      }
      if (e.key === "ArrowDown" || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "j")) {
        e.preventDefault();
        e.stopImmediatePropagation();
        paletteIndex = Math.min(filtered.length - 1, paletteIndex + 1);
        return;
      }
      if (e.key === "ArrowUp" || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k")) {
        e.preventDefault();
        e.stopImmediatePropagation();
        paletteIndex = Math.max(0, paletteIndex - 1);
        return;
      }
      return;
    }

    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "p") {
      e.preventDefault();
      e.stopPropagation();
      try {
        await openPalette(HOST_LABEL);
      } catch (err) {
        console.error("open_palette failed", err);
      }
      return;
    }

    if (e.key === "Escape" && store.isExecuting) {
      e.preventDefault();
      store.abortExecution();
    }
  }

  async function reloadActiveProvider() {
    try {
      await reloadActiveInHost(HOST_LABEL);
    } catch (err) {
      console.error("reload_active_in_host failed", err);
    }
  }

  async function selectEntry(entry: PaletteEntry) {
    if (entry.kind === "action") {
      await dismissPalette(null);
      if (entry.id === ACTION_RELOAD_ID) {
        await reloadActiveProvider();
      }
      return;
    }
    await dismissPalette(entry.id);
  }

  async function dismissPalette(selectedId: string | null) {
    try {
      await closePalette(HOST_LABEL, selectedId);
    } catch (e) {
      console.error("close_palette failed", e);
    }
  }

  async function autoShowContextIfNeeded() {
    if (contextDisabled || contextVisible) return;
    if (await hasContext()) {
      contextInitialCollapsed = true;
      contextVisible = true;
    }
  }

  function handleVoiceInput(skillId: string, text: string, autoSend: boolean) {
    const currentTab = store.tabs.find(t => t.tab_id === store.activeTabId);
    if (currentTab && currentTab.tree.current_path.length > 0) {
      store.addTab();
    }
    const inputText = skillId ? `/${skillId} ${text}` : text;
    store.updateInputText(inputText);
    if (autoSend) {
      store.sendMessage();
    }
  }

  async function applyInitParams(p: DialogInitParams) {
    if (p.new_chat) {
      store.addTab();
    } else if (p.history_entry_id) {
      await store.restoreFromHistory(p.history_entry_id, p.last_interaction_only);
    } else if (p.initial_input) {
      handleVoiceInput(p.skill_id, p.initial_input, p.auto_send_input);
    } else if (p.skill_id) {
      store.openForSkill(p.skill_id, p.skill_name, p.skill_model);
    }
  }

  async function loadModelInfo() {
    try {
      const settings = await getSettings();
      models = settings.models;
      defaultModelId = settings.surfaces.chat.generation.model_id ?? null;
    } catch {}
  }

  $effect(() => {
    const _ = filtered.length;
    if (paletteIndex >= filtered.length) {
      paletteIndex = Math.max(0, filtered.length - 1);
    }
  });

  onMount(async () => {
    window.addEventListener("keydown", handleGlobalKeydown, true);
    skillsStore.init();
    await store.initFromSettings();
    loadModelInfo();

    await refreshWebviewProviders();
    unlistenSettingsChanged = await onSettingsChanged(refreshWebviewProviders);

    const reconnected = await store.tryReconnect();

    const initParams = await invoke<DialogInitParams | null>("get_dialog_init_params");
    if (!reconnected && initParams) {
      await applyInitParams(initParams);
    }

    unlistenRestore = await listen<{ entry_id: string; last_interaction_only?: boolean }>(
      "restore-history",
      (event) => {
        store.restoreFromHistory(event.payload.entry_id, event.payload.last_interaction_only);
      },
      { target: SELF_TARGET },
    );

    unlistenVoiceInput = await listen<{ text: string; auto_send: boolean }>(
      "voice-input",
      (event) => {
        handleVoiceInput("", event.payload.text, event.payload.auto_send);
      },
      { target: SELF_TARGET },
    );

    unlistenOpenForSkill = await listen<{ skill_id: string; skill_name: string; skill_model: string | null }>(
      "open-for-skill",
      (event) => {
        store.openForSkill(event.payload.skill_id, event.payload.skill_name, event.payload.skill_model);
      },
      { target: SELF_TARGET },
    );

    unlistenNewConversation = await listen(
      "new-conversation",
      () => {
        store.addTab();
      },
      { target: SELF_TARGET },
    );

    unlistenActive = await listen<{ provider_id: string | null }>(
      "shell:active-changed",
      (ev) => {
        paletteActiveId = ev.payload.provider_id ?? PROMPTHEUS_PROVIDER_ID;
      },
      { target: SELF_TARGET },
    );

    unlistenPaletteOpened = await listen(
      "shell:palette-opened",
      async () => {
        paletteOpen = true;
        paletteQuery = "";
        paletteIndex = 0;
        await tick();
        paletteInputEl?.focus();
      },
      { target: SELF_TARGET },
    );

    unlistenPaletteClosed = await listen(
      "shell:palette-closed",
      () => {
        paletteOpen = false;
        paletteQuery = "";
      },
      { target: SELF_TARGET },
    );

    await autoShowContextIfNeeded();

    unlistenContextChanged = await listen("context-changed", () => {
      autoShowContextIfNeeded();
    });
  });

  function handleContextAutoShow() {
    if (!contextDisabled) {
      contextInitialCollapsed = !contextVisible;
      contextVisible = true;
    }
  }

  function toggleContext() {
    if (contextVisible) {
      closeContext();
    } else {
      contextVisible = true;
    }
  }

  function closeContext() {
    contextVisible = false;
    store.updateContextText("");
    store.updateContextImages([]);
  }

  async function handleSendAndCopy() {
    const currentWindow = getCurrentWindow();
    const { success, result } = await store.sendMessage();
    if (success && result) {
      await navigator.clipboard.writeText(result);
      await currentWindow.close();
    }
  }

  onDestroy(() => {
    window.removeEventListener("keydown", handleGlobalKeydown, true);
    unlistenRestore?.();
    unlistenContextChanged?.();
    unlistenVoiceInput?.();
    unlistenOpenForSkill?.();
    unlistenNewConversation?.();
    unlistenPaletteOpened?.();
    unlistenPaletteClosed?.();
    unlistenActive?.();
    unlistenSettingsChanged?.();
    store.destroy();
  });
</script>

<div class="dialog-shell">
  <div class="top-buttons" class:sidebar-open={sidebarOpen}>
    <button
      class="top-btn sidebar-toggle"
      class:hidden={sidebarOpen}
      onclick={() => sidebarOpen = !sidebarOpen}
      title="Toggle conversations"
    >
      <PanelLeft size={ICON_SIZE.md} />
    </button>
    <button
      class="top-btn"
      onclick={() => store.addTab()}
      title="New conversation"
    >
      <SquarePen size={ICON_SIZE.md} />
    </button>
  </div>
  <ConversationArea {store} />
  <InputArea {store} {models} {contextVisible} {contextDisabled} {contextInitialCollapsed} {contextWindowSize} {defaultModelId} onSendAndCopy={handleSendAndCopy} onContextAutoShow={handleContextAutoShow} onCloseContext={closeContext} onToggleContext={toggleContext} />
  <TabSidebar {store} open={sidebarOpen} onClose={() => sidebarOpen = false} />
</div>

{#if paletteOpen}
  <div class="palette-root">
    <button
      type="button"
      aria-label="Close palette"
      class="palette-scrim"
      onclick={() => dismissPalette(null)}
    ></button>
    <div class="palette-modal" role="dialog" aria-modal="true">
      <input
        bind:this={paletteInputEl}
        bind:value={paletteQuery}
        oninput={() => (paletteIndex = 0)}
        class="palette-input"
        type="text"
        placeholder="Search providers and actions..."
        autocomplete="off"
        spellcheck="false"
      />
      <div class="palette-list" role="listbox">
        {#each filtered as entry, i (entry.id)}
          {#if entry.kind === "action" && i > 0 && filtered[i - 1].kind === "provider"}
            <div class="palette-divider" role="separator"></div>
          {/if}
          <button
            type="button"
            role="option"
            aria-selected={i === paletteIndex}
            class="palette-item"
            class:highlight={i === paletteIndex}
            onmouseenter={() => (paletteIndex = i)}
            onclick={() => selectEntry(entry)}
          >
            <span class="palette-item-icon" aria-hidden="true">
              {#if entry.kind === "provider"}
                {@const iconSvg = providerIconSvg(entry)}
                {#if iconSvg}
                  {@html iconSvg}
                {/if}
              {:else}
                <RefreshCw size={14} />
              {/if}
            </span>
            <span class="palette-item-name">{entry.name}</span>
            {#if entry.kind === "provider" && entry.id === paletteActiveId}
              <span class="palette-item-badge">active</span>
            {/if}
          </button>
        {:else}
          <div class="palette-empty">no matches</div>
        {/each}
      </div>
      <div class="palette-footer">
        <span>↑↓ / ⌃jk navigate</span>
        <span>↵ select</span>
        <span>⌘R reload</span>
        <span>esc close</span>
      </div>
    </div>
  </div>
{/if}

<style>
  :global(html),
  :global(body) {
    background: #1e1e1e;
  }

  .dialog-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #1e1e1e;
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 13px;
    overflow: hidden;
    position: relative;
  }

  .top-buttons {
    position: absolute;
    top: 6px;
    left: 6px;
    z-index: 201;
    display: flex;
    gap: 4px;
    transition: transform 0.2s ease;
  }

  .top-buttons.sidebar-open {
    transform: translateX(240px);
  }

  .sidebar-toggle {
    width: 28px;
    overflow: visible;
    transition: width 0.2s ease, opacity 0.2s ease;
  }

  .sidebar-toggle.hidden {
    width: 0;
    opacity: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .top-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: none;
    background: rgba(255, 255, 255, 0.03);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    color: rgba(255, 255, 255, 0.35);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    position: relative;
  }

  :global([data-platform="linux"]) .top-btn {
    background: rgba(255, 255, 255, 0.06);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }

  .top-btn:hover {
    color: rgba(255, 255, 255, 0.8);
    background: rgba(255, 255, 255, 0.1);
  }

  .palette-root {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 80px;
    z-index: 1000;
  }

  .palette-scrim {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    border: 0;
    padding: 0;
    cursor: default;
  }

  .palette-modal {
    position: relative;
    width: min(520px, 80%);
    background: #252525;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    color: #e0e0e0;
  }

  .palette-input {
    appearance: none;
    border: 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    background: transparent;
    color: #fff;
    font: inherit;
    font-size: 14px;
    padding: 12px 14px;
    outline: none;
  }

  .palette-list {
    display: flex;
    flex-direction: column;
    max-height: 320px;
    overflow-y: auto;
  }

  .palette-item {
    appearance: none;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.85);
    font: inherit;
    text-align: left;
    padding: 8px 14px;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .palette-item.highlight {
    background: rgba(255, 255, 255, 0.08);
  }

  .palette-item-icon {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.85);
  }

  .palette-item-icon :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
  }

  .palette-item-icon :global(img) {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: contain;
  }

  .palette-item-name {
    flex: 1;
    font-size: 13px;
  }

  .palette-item-badge {
    font-size: 10px;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.45);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 3px;
    padding: 1px 5px;
  }

  .palette-empty {
    color: rgba(255, 255, 255, 0.4);
    padding: 16px;
    text-align: center;
  }

  .palette-divider {
    height: 1px;
    background: rgba(255, 255, 255, 0.06);
    margin: 4px 0;
  }

  .palette-footer {
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    padding: 6px 14px;
    display: flex;
    gap: 12px;
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
  }
</style>
