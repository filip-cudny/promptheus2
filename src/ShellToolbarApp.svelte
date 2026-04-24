<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { Minus, Plus, Square, SquareArrowOutUpRight, X } from "lucide-svelte";
  import {
    getAiProviders,
    openAiWebviewNewWindow,
    swapAiWebview,
    swapToConversationDialog,
    type AiProvider,
  } from "$lib/services/aiWebview";
  import {
    CONVERSATION_DIALOG_LABEL,
    PROMPTHEUS_PROVIDER_ID,
    getActiveProvider,
    newChatInHost,
    openPalette,
  } from "$lib/services/shellToolbar";

  const HOST_LABEL = CONVERSATION_DIALOG_LABEL;
  const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);
  const shortcutHint = isMac ? "⌘P" : "Ctrl P";

  let aiProviders = $state<AiProvider[]>([]);
  let activeId = $state<string>(PROMPTHEUS_PROVIDER_ID);
  let isMaximized = $state(false);

  let providers = $derived<{ id: string; name: string }[]>([
    { id: PROMPTHEUS_PROVIDER_ID, name: "Promptheus" },
    ...aiProviders.map((p) => ({ id: p.id, name: p.name })),
  ]);

  let unlistenActive: UnlistenFn | undefined;

  async function refreshActive() {
    try {
      const pid = await getActiveProvider(HOST_LABEL);
      activeId = pid ?? PROMPTHEUS_PROVIDER_ID;
    } catch (e) {
      console.error("get_active_provider failed", e);
    }
  }

  async function selectProvider(id: string) {
    if (id === activeId) return;
    try {
      if (id === PROMPTHEUS_PROVIDER_ID) {
        await swapToConversationDialog(HOST_LABEL);
      } else {
        await swapAiWebview(id, HOST_LABEL);
      }
    } catch (e) {
      console.error("shell toolbar swap failed", e);
    }
  }

  async function handleNewChat() {
    if (activeId === PROMPTHEUS_PROVIDER_ID) return;
    try {
      await newChatInHost(HOST_LABEL);
    } catch (e) {
      console.error("new chat failed", e);
    }
  }

  async function handleOpenInNewWindow() {
    if (activeId === PROMPTHEUS_PROVIDER_ID) return;
    try {
      await openAiWebviewNewWindow(activeId);
    } catch (e) {
      console.error("open in new window failed", e);
    }
  }

  async function handleMinimize() {
    try {
      await getCurrentWindow().minimize();
    } catch (e) {
      console.error("minimize failed", e);
    }
  }

  async function handleToggleMaximize() {
    try {
      await getCurrentWindow().toggleMaximize();
      isMaximized = await getCurrentWindow().isMaximized();
    } catch (e) {
      console.error("toggle maximize failed", e);
    }
  }

  async function handleClose() {
    try {
      await getCurrentWindow().close();
    } catch (e) {
      console.error("close failed", e);
    }
  }

  async function handleGlobalKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "p") {
      e.preventDefault();
      e.stopPropagation();
      try {
        await openPalette(HOST_LABEL);
      } catch (err) {
        console.error("open_palette failed", err);
      }
    }
  }

  onMount(async () => {
    window.addEventListener("keydown", handleGlobalKeydown, true);

    try {
      aiProviders = await getAiProviders();
    } catch (e) {
      console.error("getAiProviders failed", e);
    }
    await refreshActive();

    try {
      isMaximized = await getCurrentWindow().isMaximized();
    } catch {}

    unlistenActive = await listen<{ provider_id: string | null }>(
      "shell:active-changed",
      (ev) => {
        activeId = ev.payload.provider_id ?? PROMPTHEUS_PROVIDER_ID;
      },
    );
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleGlobalKeydown, true);
    unlistenActive?.();
  });
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="switcher" role="tablist">
    {#each providers as p (p.id)}
      <button
        type="button"
        role="tab"
        aria-selected={activeId === p.id}
        class="tab"
        class:active={activeId === p.id}
        onclick={() => selectProvider(p.id)}
      >
        {p.name}
      </button>
    {/each}
  </div>

  <div class="drag-fill" data-tauri-drag-region></div>

  <div class="actions">
    <span class="hint" title="Open command palette">{shortcutHint}</span>
    <button
      type="button"
      class="icon-btn"
      title="New chat"
      disabled={activeId === PROMPTHEUS_PROVIDER_ID}
      onclick={handleNewChat}
    >
      <Plus size={14} />
    </button>
    <button
      type="button"
      class="icon-btn"
      title="Open in new window"
      disabled={activeId === PROMPTHEUS_PROVIDER_ID}
      onclick={handleOpenInNewWindow}
    >
      <SquareArrowOutUpRight size={14} />
    </button>

    <div class="sep"></div>

    <button type="button" class="win-btn" title="Minimize" onclick={handleMinimize}>
      <Minus size={14} />
    </button>
    <button type="button" class="win-btn" title={isMaximized ? "Restore" : "Maximize"} onclick={handleToggleMaximize}>
      <Square size={12} />
    </button>
    <button type="button" class="win-btn close" title="Close" onclick={handleClose}>
      <X size={14} />
    </button>
  </div>
</div>

<style>
  :global(html),
  :global(body) {
    background: #1e1e1e;
  }

  .titlebar {
    height: 40px;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 4px 0 8px;
    background: #1e1e1e;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 12px;
    user-select: none;
    -webkit-user-select: none;
    box-sizing: border-box;
  }

  .switcher {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.04);
  }

  .tab {
    appearance: none;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.55);
    padding: 4px 10px;
    border-radius: 4px;
    font: inherit;
    cursor: pointer;
    line-height: 1;
  }

  .tab:hover {
    color: rgba(255, 255, 255, 0.9);
    background: rgba(255, 255, 255, 0.04);
  }

  .tab.active {
    color: #fff;
    background: rgba(255, 255, 255, 0.12);
  }

  .drag-fill {
    flex: 1;
    align-self: stretch;
  }

  .actions {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .hint {
    color: rgba(255, 255, 255, 0.35);
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    margin-right: 4px;
  }

  .icon-btn {
    appearance: none;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    width: 26px;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    cursor: pointer;
  }

  .icon-btn:hover:not(:disabled) {
    color: #fff;
    background: rgba(255, 255, 255, 0.08);
  }

  .icon-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .sep {
    width: 1px;
    height: 20px;
    background: rgba(255, 255, 255, 0.08);
    margin: 0 4px;
  }

  .win-btn {
    appearance: none;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.55);
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    cursor: pointer;
  }

  .win-btn:hover {
    color: #fff;
    background: rgba(255, 255, 255, 0.08);
  }

  .win-btn.close:hover {
    background: #e81123;
    color: #fff;
  }
</style>
