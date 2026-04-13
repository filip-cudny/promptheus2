<script lang="ts">
  import { onMount, onDestroy, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ContextSection from "./ContextSection.svelte";
  import AttachMenu from "./AttachMenu.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import TextChipBar from "$lib/components/ui/TextChipBar.svelte";
  import SkillEditable from "$lib/components/ui/SkillEditable.svelte";
  import ModelSelector from "$lib/components/ui/ModelSelector.svelte";
  import ToolChip from "./ToolChip.svelte";
  import { SendHorizonal, RefreshCw, Square, CopyCheck, Globe } from "lucide-svelte";
  import type { ComponentType, SvelteComponent } from "svelte";
  import type { IconProps } from "lucide-svelte";

  type LucideIcon = ComponentType<SvelteComponent<IconProps>>;
  import { getImageFromPasteEvent, extractTextAttachment } from "$lib/utils/paste";
  import { formatTokenCount } from "$lib/utils/contextWindow";
  import { ICON_SIZE, TEXT_ATTACHMENT_CHAR_THRESHOLD } from "$lib/constants/ui";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import type { createConversationStore } from "$lib/stores/conversation.svelte";
  import type { ConversationImage } from "$lib/types/conversation";
  import type { ModelConfig } from "$lib/types";

  let {
    store,
    models = [],
    contextVisible,
    contextDisabled,
    contextInitialCollapsed = false,
    contextWindowSize = 0,
    onSendAndCopy,
    onContextAutoShow,
    onCloseContext,
    onToggleContext,
    defaultModelId = null,
  }: {
    store: ReturnType<typeof createConversationStore>;
    models?: ModelConfig[];
    contextVisible: boolean;
    contextDisabled: boolean;
    contextInitialCollapsed?: boolean;
    contextWindowSize?: number;
    onSendAndCopy: () => void;
    onContextAutoShow: () => void;
    onCloseContext: () => void;
    onToggleContext: () => void;
    defaultModelId?: string | null;
  } = $props();

  let mcpWebSearchQualifiedId = $state<string | null>(null);

  const activeModel = $derived.by(() => {
    const activeModelId = store.modelId ?? defaultModelId;
    if (!activeModelId) return null;
    return models.find((m) => m.id === activeModelId) ?? null;
  });

  const builtinWebSearchAvailable = $derived(
    activeModel?.enabled_tools?.includes("web_search") ?? false,
  );

  const mcpWebSearchAvailable = $derived.by(() => {
    if (!mcpWebSearchQualifiedId || !activeModel) return false;
    const tools = activeModel.enabled_tools ?? [];
    return tools.includes(mcpWebSearchQualifiedId);
  });

  const webSearchAvailable = $derived(builtinWebSearchAvailable || mcpWebSearchAvailable);
  const bothWebSearchAvailable = $derived(builtinWebSearchAvailable && mcpWebSearchAvailable);

  const availableTools = $derived.by(() => {
    const tools: { id: string; label: string; icon: LucideIcon; active: boolean }[] = [];
    if (bothWebSearchAvailable) {
      tools.push({ id: "web_search", label: "Web Search (Built-in)", icon: Globe, active: store.selectedTools.includes("web_search") });
      tools.push({ id: mcpWebSearchQualifiedId!, label: "Web Search (MCP)", icon: Globe, active: store.selectedTools.includes(mcpWebSearchQualifiedId!) });
    } else if (webSearchAvailable) {
      const toolId = mcpWebSearchAvailable && mcpWebSearchQualifiedId ? mcpWebSearchQualifiedId : "web_search";
      tools.push({ id: toolId, label: "Web Search", icon: Globe, active: store.selectedTools.includes(toolId) });
    }
    return tools;
  });

  function handleToggleTool(id: string, enabled: boolean) {
    if (enabled && bothWebSearchAvailable) {
      const otherId = id === "web_search" ? mcpWebSearchQualifiedId : "web_search";
      if (otherId && store.selectedTools.includes(otherId)) {
        store.toggleTool(otherId, false);
      }
    }
    store.toggleTool(id, enabled);
  }

  let localText = $state("");
  let localImages = $state<ConversationImage[]>([]);
  let localTextAttachments = $state<string[]>([]);
  let shiftHeld = $state(false);
  let skillEditable: ReturnType<typeof SkillEditable> | undefined = $state();
  let syncedTabId = $state("");
  let lastDomText = $state("");

  $effect(() => {
    if (store.activeTabId === syncedTabId) {
      store.updateInputText(localText);
      lastDomText = localText;
    }
  });

  $effect(() => {
    if (store.activeTabId === syncedTabId) {
      store.updateInputImages(localImages);
    }
  });

  $effect(() => {
    if (store.activeTabId === syncedTabId) {
      store.updateInputTextAttachments(localTextAttachments);
    }
  });

  $effect(() => {
    const tabId = store.activeTabId;
    const storeText = store.inputText;
    const storeImages = store.inputImages;
    const storeAttachments = store.inputTextAttachments;

    const tabChanged = tabId !== untrack(() => syncedTabId);
    const textChangedExternally = storeText !== untrack(() => lastDomText);

    localText = storeText;
    localImages = storeImages;
    localTextAttachments = storeAttachments;
    syncedTabId = tabId;

    if (skillEditable) {
      if (storeText === "") {
        skillEditable.setTextAndHighlight("");
        skillEditable.resetUndoStack("");
        lastDomText = "";
      } else if (tabChanged || textChangedExternally) {
        skillEditable.setTextAndHighlight(storeText);
        skillEditable.resetUndoStack(storeText);
        lastDomText = storeText;
        requestAnimationFrame(() => {
          skillEditable?.focus();
          skillEditable?.restoreCursor(storeText.length);
        });
      }
    }
  });

  let unlistenTextUpdate: (() => void) | null = null;

  function onKeyDown(e: KeyboardEvent) { shiftHeld = e.shiftKey; }
  function onKeyUp(e: KeyboardEvent) { shiftHeld = e.shiftKey; }

  onMount(async () => {
    skillEditable?.focus();
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);

    invoke<{ name: string; server: string }[]>("list_mcp_tools")
      .then((tools) => {
        const ws = tools.find((t) => t.name === "web_search");
        mcpWebSearchQualifiedId = ws ? `${ws.server}.${ws.name}` : null;
      })
      .catch(() => {});

    const win = getCurrentWebviewWindow();
    unlistenTextUpdate = await win.listen<{ text: string; index: number }>(
      "text-attachment-updated",
      (event) => {
        const { text, index } = event.payload;
        if (index >= 0 && index < localTextAttachments.length) {
          localTextAttachments = localTextAttachments.map((t, i) =>
            i === index ? text : t,
          );
        }
      },
    );
  });

  onDestroy(() => {
    unlistenTextUpdate?.();
    window.removeEventListener("keydown", onKeyDown);
    window.removeEventListener("keyup", onKeyUp);
  });

  function sendOrRegenerate() {
    const abortNodeId = store.abortRegenerateNodeId;
    if (abortNodeId) {
      store.editAndRegenerate(abortNodeId, localText.trim());
      return;
    }
    if (store.isRegenerateMode) {
      const path = store.tree.current_path;
      if (path.length > 0) {
        store.regenerate(path[path.length - 1]);
      }
    } else if (store.canSend) {
      store.sendMessage();
    }
  }

  async function handleKeydown(e: KeyboardEvent) {
    if (e.key.toLowerCase() === "v" && e.shiftKey && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      try {
        const text = await invoke<string>("get_clipboard_text");
        if (text) {
          document.execCommand("insertText", false, text);
        }
      } catch {}
      return;
    }

    if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      sendOrRegenerate();
      return;
    }

    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      if (store.canSend) {
        onSendAndCopy();
      }
      return;
    }
  }

  async function handlePaste(e: ClipboardEvent) {
    const textAttachment = !shiftHeld ? extractTextAttachment(e, TEXT_ATTACHMENT_CHAR_THRESHOLD) : null;
    if (textAttachment) {
      requestAnimationFrame(() => {
        localTextAttachments = [...localTextAttachments, textAttachment];
      });
      return;
    }

    const plainText = e.clipboardData?.getData("text/plain") ?? "";
    e.preventDefault();

    if (plainText) {
      document.execCommand("insertText", false, plainText);
      return;
    }

    const image = await getImageFromPasteEvent(e);
    if (image) {
      localImages = [...localImages, image];
    }
  }

  function handleInputAreaClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target.closest("button, .attach-menu, .model-selector, .autocomplete-dropdown, .context-section, .attachment-row")) return;
    skillEditable?.focus();
  }
</script>

<div class="input-area" onclick={handleInputAreaClick}>
  {#if contextVisible}
    <ContextSection {store} {contextDisabled} initialCollapsed={contextInitialCollapsed} onHasContent={onContextAutoShow} onClose={onCloseContext} />
  {/if}

  <div class="input-field">
    {#if localTextAttachments.length > 0 || localImages.length > 0}
      <div class="attachment-row">
        <TextChipBar bind:textAttachments={localTextAttachments} readonly={false} variant="small" />
        <ImageChipBar bind:images={localImages} readonly={false} variant="small" />
      </div>
    {/if}
    <SkillEditable
      bind:this={skillEditable}
      bind:text={localText}
      placeholder="Type a message… (use /skill-name for skills)"
      editableClass="input-editable"
      onkeydown={handleKeydown}
      onpaste={handlePaste}
    />
  </div>

  <div class="button-bar">
    <div class="bar-left">
      <AttachMenu
        onSelectContext={onToggleContext}
        {contextDisabled}
        {availableTools}
        onToggleTool={handleToggleTool}
      />
      {#each availableTools.filter(t => t.active) as tool (tool.id)}
        <ToolChip label={tool.label} icon={tool.icon} ondismiss={() => store.toggleTool(tool.id, false)} />
      {/each}
    </div>

    <div class="bar-right">
      {#if store.totalTokens > 0}
        <span class="token-count" title="Context tokens">
          ~{formatTokenCount(store.totalTokens)}{#if contextWindowSize > 0}&nbsp;/ {formatTokenCount(contextWindowSize)}{/if}
        </span>
      {/if}
      {#if models.length > 0}
        <ModelSelector
          {models}
          selectedModelId={store.modelId}
          reasoningEffort={store.reasoningEffort}
          onModelSelect={(modelId) => store.updateModelId(modelId)}
          onReasoningSelect={(effort) => store.updateReasoningEffort(effort)}
        />
      {/if}

      <ActionIconButton
        icon={CopyCheck}
        onclick={onSendAndCopy}
        disabled={!store.canSend || store.isExecuting}
        title="Send & Copy (Ctrl+Enter)"
      />

      {#if store.isExecuting}
        <ActionIconButton
          icon={Square}
          onclick={() => store.stopExecution()}
          title="Stop"
        />
      {:else if store.abortRegenerateNodeId || store.isRegenerateMode}
        <ActionIconButton
          icon={RefreshCw}
          onclick={sendOrRegenerate}
          title="Regenerate (Enter)"
        />
      {:else}
        <ActionIconButton
          icon={SendHorizonal}
          onclick={sendOrRegenerate}
          disabled={!store.canSend}
          title="Send (Enter)"
        />
      {/if}
    </div>
  </div>
</div>

<style>
  .input-area {
    flex-shrink: 0;
    position: relative;
    z-index: 10;
    margin: -8px 16px 8px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 8px;
    background: rgba(30, 30, 30, 0.75);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    overflow: visible;
  }

  :global([data-platform="linux"]) .input-area {
    background: rgba(30, 30, 30, 0.95);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }

  .input-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 8px 0;
  }

  .attachment-row {
    display: flex;
    flex-wrap: nowrap;
    gap: 6px;
    padding: 6px 0 2px;
    overflow-x: auto;
  }

  .input-field :global(.input-editable) {
    font-size: 13px;
    max-height: 35vh;
    overflow-y: auto;
    padding: 4px 2px;
  }

  .button-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 8px;
    position: relative;
    overflow: visible;
  }

  .bar-left {
    flex-shrink: 0;
    display: flex;
    gap: 6px;
    align-items: center;
    position: relative;
    overflow: visible;
  }

  .token-count {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.4);
    user-select: none;
    white-space: nowrap;
    margin-right: 2px;
  }

  .bar-right {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 6px;
  }

</style>
