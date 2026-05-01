<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import ContextSection from "./ContextSection.svelte";
  import AttachMenu from "./AttachMenu.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import SkillEditable from "$lib/components/ui/SkillEditable.svelte";
  import ModelSelector from "$lib/components/ui/ModelSelector.svelte";
  import ToolChip from "./ToolChip.svelte";
  import AttachmentRow from "./components/AttachmentRow.svelte";
  import { prefetchCapabilities, getCachedCapabilities } from "$lib/stores/capabilities.svelte";
  import { SendHorizonal, RefreshCw, Square, CopyCheck, Globe } from "lucide-svelte";
  import type { ComponentType, SvelteComponent } from "svelte";
  import type { IconProps } from "lucide-svelte";

  type LucideIcon = ComponentType<SvelteComponent<IconProps>>;
  import { handleEditablePaste } from "$lib/utils/paste";
  import { formatTokenCount } from "$lib/utils/contextWindow";
  import { focusConversationInput, setConversationInputFocus } from "$lib/utils/conversationFocus";
  import { ICON_SIZE } from "$lib/constants/ui";
  import type { createConversationStore } from "$lib/stores/conversation.svelte";
  import type { ConversationImage } from "$lib/types/conversation";
  import type { ModelConfig } from "$lib/types";
  import { openImagePreview, openTextPreview } from "$lib/services/windowPreviews";
  import { readClipboardForEditable } from "$lib/services/clipboardPaste";
  import { useInputSync } from "./drivers/useInputSync.svelte";
  import { useMcpTools } from "./drivers/useMcpTools.svelte";
  import { useTextAttachmentBridge } from "./drivers/useTextAttachmentBridge.svelte";

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

  let skillEditable: ReturnType<typeof SkillEditable> | undefined = $state();

  const sync = useInputSync({ store, getSkillEditable: () => skillEditable });
  const mcp = useMcpTools();
  const attachmentBridge = useTextAttachmentBridge({
    getAttachments: () => sync.localTextAttachments,
    setAttachments: (next) => {
      sync.localTextAttachments = next;
    },
  });

  let unlistenMcp: (() => void) | undefined;
  let unlistenAttachmentBridge: (() => void) | undefined;
  let shiftHeld = $state(false);

  const activeModel = $derived.by(() => {
    const activeModelId = store.modelId ?? defaultModelId;
    if (!activeModelId) return null;
    return models.find((m) => m.id === activeModelId) ?? null;
  });

  $effect(() => { prefetchCapabilities(activeModel); });
  const activeModelCapabilities = $derived(getCachedCapabilities(activeModel));

  const builtinWebSearchAvailable = $derived(activeModel?.provider === "openai");
  const mcpWebSearchAvailable = $derived(!!mcp.webSearchQualifiedId && !!activeModel);
  const webSearchAvailable = $derived(builtinWebSearchAvailable || mcpWebSearchAvailable);
  const bothWebSearchAvailable = $derived(builtinWebSearchAvailable && mcpWebSearchAvailable);

  const availableTools = $derived.by(() => {
    const tools: { id: string; label: string; icon: LucideIcon; active: boolean }[] = [];
    if (bothWebSearchAvailable) {
      tools.push({ id: "web_search", label: "Web Search (Built-in)", icon: Globe, active: store.selectedTools.includes("web_search") });
      tools.push({ id: mcp.webSearchQualifiedId!, label: "Web Search (MCP)", icon: Globe, active: store.selectedTools.includes(mcp.webSearchQualifiedId!) });
    } else if (webSearchAvailable) {
      const toolId = mcpWebSearchAvailable && mcp.webSearchQualifiedId ? mcp.webSearchQualifiedId : "web_search";
      tools.push({ id: toolId, label: "Web Search", icon: Globe, active: store.selectedTools.includes(toolId) });
    }
    return tools;
  });

  function handleToggleTool(id: string, enabled: boolean) {
    if (enabled && bothWebSearchAvailable) {
      const otherId = id === "web_search" ? mcp.webSearchQualifiedId : "web_search";
      if (otherId && store.selectedTools.includes(otherId)) {
        store.toggleTool(otherId, false);
      }
    }
    store.toggleTool(id, enabled);
    focusConversationInput();
  }

  function handleSelectModel(modelId: string) {
    store.updateModelId(modelId);
    focusConversationInput();
  }

  function handleSelectReasoning(effort: string | null) {
    store.updateReasoningEffort(effort);
    focusConversationInput();
  }

  function handleRemoveTextAttachment(idx: number) {
    sync.localTextAttachments = sync.localTextAttachments.filter((_, i) => i !== idx);
    focusConversationInput();
  }

  function handleRemoveImage(idx: number) {
    sync.localImages = sync.localImages.filter((_, i) => i !== idx);
    focusConversationInput();
  }

  function handleDismissTool(id: string) {
    store.toggleTool(id, false);
    focusConversationInput();
  }

  const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);

  function onKeyDown(e: KeyboardEvent) { shiftHeld = e.shiftKey; }
  function onKeyUp(e: KeyboardEvent) { shiftHeld = e.shiftKey; }

  onMount(async () => {
    setConversationInputFocus(() => skillEditable?.focus());
    skillEditable?.focus();
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);

    unlistenMcp = await mcp.init();
    unlistenAttachmentBridge = await attachmentBridge.init();
  });

  onDestroy(() => {
    setConversationInputFocus(null);
    unlistenAttachmentBridge?.();
    unlistenMcp?.();
    window.removeEventListener("keydown", onKeyDown);
    window.removeEventListener("keyup", onKeyUp);
  });

  function sendOrRegenerate() {
    const abortNodeId = store.abortRegenerateNodeId;
    if (abortNodeId) {
      store.editAndRegenerate(abortNodeId, sync.localText.trim());
      return;
    }
    if (store.isRegenerateMode) {
      const path = store.tree.current_path;
      if (path.length > 0) store.regenerate(path[path.length - 1]);
    } else if (store.canSend) {
      store.sendMessage();
    }
  }

  async function handleKeydown(e: KeyboardEvent) {
    if (isMac && e.key.toLowerCase() === "v" && e.shiftKey && e.metaKey) {
      e.preventDefault();
      const result = await readClipboardForEditable();
      if (result?.kind === "text") {
        document.execCommand("insertText", false, result.text);
      } else if (result?.kind === "image") {
        sync.localImages = [...sync.localImages, { data: result.data, media_type: result.mediaType }];
      }
      return;
    }

    if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      sendOrRegenerate();
      return;
    }

    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      if (store.canSend) onSendAndCopy();
      return;
    }
  }

  async function handlePaste(e: ClipboardEvent) {
    await handleEditablePaste(e, {
      skipTextAttachment: shiftHeld,
      onTextAttachment: (text) => { sync.localTextAttachments = [...sync.localTextAttachments, text]; },
      onImage: (image) => { sync.localImages = [...sync.localImages, image]; },
    });
  }

  function handleInputAreaClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target.closest("button, .attach-menu, .model-selector, .autocomplete-dropdown, .context-section, .attachment-row")) return;
    skillEditable?.focus();
  }

  function handleOpenImage(image: ConversationImage) {
    openImagePreview(image.data, image.media_type);
  }
</script>

<div class="input-area" onclick={handleInputAreaClick}>
  {#if contextVisible}
    <ContextSection {store} {contextDisabled} initialCollapsed={contextInitialCollapsed} onHasContent={onContextAutoShow} onClose={onCloseContext} />
  {/if}

  <div class="input-field">
    <AttachmentRow
      textAttachments={sync.localTextAttachments}
      images={sync.localImages}
      variant="small"
      onRemoveText={handleRemoveTextAttachment}
      onRemoveImage={handleRemoveImage}
      onOpenText={openTextPreview}
      onOpenImage={handleOpenImage}
    />
    <SkillEditable
      bind:this={skillEditable}
      bind:text={sync.localText}
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
        <ToolChip label={tool.label} icon={tool.icon} ondismiss={() => handleDismissTool(tool.id)} />
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
          capabilities={activeModelCapabilities}
          onModelSelect={handleSelectModel}
          onReasoningSelect={handleSelectReasoning}
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
    z-index: var(--z-sticky);
    margin: -8px var(--space-8) var(--space-4);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xl);
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
    gap: var(--space-2);
    padding: var(--space-4) var(--space-4) var(--space-0);
  }

  .input-field :global(.input-editable) {
    font-size: var(--font-size-base);
    max-height: 35vh;
    overflow-y: auto;
    padding: var(--space-2) var(--space-1);
  }

  .button-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4);
    position: relative;
    overflow: visible;
  }

  .bar-left {
    flex-shrink: 0;
    display: flex;
    gap: var(--space-3);
    align-items: center;
    position: relative;
    overflow: visible;
  }

  .token-count {
    font-size: var(--font-size-sm);
    color: var(--text-disabled);
    user-select: none;
    white-space: nowrap;
    margin-right: var(--space-1);
  }

  .bar-right {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
</style>
