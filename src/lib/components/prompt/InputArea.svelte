<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ContextSection from "./ContextSection.svelte";
  import AttachMenu from "./AttachMenu.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import { SendHorizonal, RefreshCw, Square, CopyCheck } from "lucide-svelte";
  import { getImageFromPasteEvent } from "$lib/utils/paste";
  import { autoResize, resizeTextarea } from "$lib/utils/autoResize";
  import { listSkills } from "$lib/services/skills";
  import type { createConversationStore } from "$lib/stores/conversation.svelte";
  import type { ConversationImage } from "$lib/types/conversation";
  import type { SkillSummary } from "$lib/types";

  let {
    store,
    contextVisible,
    contextDisabled,
    contextInitialCollapsed = false,
    onSendAndCopy,
    onContextAutoShow,
    onCloseContext,
    onToggleContext,
  }: {
    store: ReturnType<typeof createConversationStore>;
    contextVisible: boolean;
    contextDisabled: boolean;
    contextInitialCollapsed?: boolean;
    onSendAndCopy: () => void;
    onContextAutoShow: () => void;
    onCloseContext: () => void;
    onToggleContext: () => void;
  } = $props();

  let textarea: HTMLTextAreaElement | undefined = $state();
  let localText = $state("");
  let localImages = $state<ConversationImage[]>([]);

  let allSkills = $state<SkillSummary[]>([]);
  let showAutocomplete = $state(false);
  let autocompleteItems = $state<SkillSummary[]>([]);
  let autocompleteIndex = $state(0);
  let slashStart = $state(-1);

  $effect(() => {
    store.updateInputText(localText);
  });

  $effect(() => {
    store.updateInputImages(localImages);
  });

  $effect(() => {
    if (store.inputText === "" && localText !== "") {
      localText = "";
    }
  });

  $effect(() => {
    if (store.inputImages.length === 0 && localImages.length > 0) {
      localImages = [];
    }
  });

  onMount(async () => {
    textarea?.focus();
    try {
      allSkills = await listSkills();
    } catch {
      allSkills = [];
    }
  });

  $effect(() => {
    localText;
    if (textarea) requestAnimationFrame(() => resizeTextarea(textarea!));
  });

  function highlightInput(text: string): string {
    if (!text) return "\n";
    return text
      .split("\n")
      .map((line) => {
        const match = line.match(/^(\/[a-z0-9-]+)(\s.*)?$/);
        if (match) {
          const badge = `<span class="hl-skill">${match[1]}</span>`;
          const rest = match[2] ?? "";
          return badge + rest;
        }
        return line;
      })
      .join("\n") + "\n";
  }

  function detectSlashCommand() {
    if (!textarea) {
      closeAutocomplete();
      return;
    }

    const pos = textarea.selectionStart;
    const textBefore = localText.slice(0, pos);
    const lastNewline = textBefore.lastIndexOf("\n");
    const lineStart = lastNewline + 1;
    const lineText = textBefore.slice(lineStart);

    const match = lineText.match(/^\/([a-z0-9-]*)$/);
    if (match && match[1] !== undefined) {
      const query = match[1];
      slashStart = lineStart;
      const filtered = allSkills.filter(
        (s) =>
          s.name.includes(query) ||
          s.display_name.toLowerCase().includes(query),
      );
      if (filtered.length > 0) {
        autocompleteItems = filtered;
        autocompleteIndex = 0;
        showAutocomplete = true;
        return;
      }
    }

    closeAutocomplete();
  }

  function closeAutocomplete() {
    showAutocomplete = false;
    autocompleteItems = [];
    autocompleteIndex = 0;
    slashStart = -1;
  }

  function insertSkill(skill: SkillSummary) {
    if (!textarea || slashStart < 0) return;

    const pos = textarea.selectionStart;
    const before = localText.slice(0, slashStart);
    const after = localText.slice(pos);
    localText = `${before}/${skill.name} ${after}`;
    closeAutocomplete();

    requestAnimationFrame(() => {
      if (!textarea) return;
      const newPos = slashStart + skill.name.length + 2;
      textarea.selectionStart = newPos;
      textarea.selectionEnd = newPos;
      textarea.focus();
    });
  }

  function handleSendShow() {
    if (store.isRegenerateMode) {
      const path = store.tree.current_path;
      if (path.length > 0) {
        store.regenerate(path[path.length - 1]);
      }
    } else {
      store.sendMessage();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (showAutocomplete) {
        e.preventDefault();
        closeAutocomplete();
        return;
      }
      e.preventDefault();
      getCurrentWindow().close();
      return;
    }

    if (showAutocomplete) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        autocompleteIndex = (autocompleteIndex + 1) % autocompleteItems.length;
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        autocompleteIndex =
          (autocompleteIndex - 1 + autocompleteItems.length) %
          autocompleteItems.length;
        return;
      }
      if (e.key === "Tab" || (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey)) {
        e.preventDefault();
        if (autocompleteItems.length > 0) {
          insertSkill(autocompleteItems[autocompleteIndex]);
        }
        return;
      }
    }

    if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      if (store.isRegenerateMode) {
        const path = store.tree.current_path;
        if (path.length > 0) {
          store.regenerate(path[path.length - 1]);
        }
      } else if (store.canSend) {
        store.sendMessage();
      }
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

  function handleInput() {
    detectSlashCommand();
  }

  async function handlePaste(e: ClipboardEvent) {
    const image = await getImageFromPasteEvent(e);
    if (image) {
      localImages = [...localImages, image];
    }
  }
</script>

<div class="input-area">
  {#if contextVisible}
    <ContextSection {store} {contextDisabled} initialCollapsed={contextInitialCollapsed} onHasContent={onContextAutoShow} onClose={onCloseContext} />
  {/if}

  <div class="input-field">
    <ImageChipBar bind:images={localImages} readonly={false} />
    <div class="textarea-wrapper">
      <div class="input-highlight" aria-hidden="true">{@html highlightInput(localText)}</div>
      <textarea
        bind:this={textarea}
        class="input-textarea"
        bind:value={localText}
        use:autoResize={"40vh"}
        rows="1"
        placeholder="Type a message… (use /skill-name for skills)"
        onkeydown={handleKeydown}
        oninput={handleInput}
        onpaste={handlePaste}
        disabled={store.isExecuting}
      ></textarea>

      {#if showAutocomplete && autocompleteItems.length > 0}
        <div class="autocomplete-dropdown">
          {#each autocompleteItems as item, i}
            <button
              class="autocomplete-item"
              class:selected={i === autocompleteIndex}
              onmousedown={(e) => { e.preventDefault(); insertSkill(item); }}
              onmouseenter={() => autocompleteIndex = i}
            >
              <span class="autocomplete-name">/{item.name}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <div class="button-bar">
    <div class="bar-left">
      <AttachMenu onSelectContext={onToggleContext} {contextDisabled} />
    </div>

    <div class="bar-right">
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
      {:else if store.isRegenerateMode}
        <ActionIconButton
          icon={RefreshCw}
          onclick={handleSendShow}
          title="Regenerate"
        />
      {:else}
        <ActionIconButton
          icon={SendHorizonal}
          onclick={handleSendShow}
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

  .input-textarea {
    position: relative;
    width: 100%;
    min-height: 0;
    background: transparent;
    border: none;
    color: #e0e0e0;
    font: inherit;
    font-size: 13px;
    padding: 4px 2px;
    box-sizing: border-box;
    z-index: 1;
  }

  .input-textarea:focus {
    outline: none;
  }

  .input-textarea:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .button-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 8px;
  }

  .bar-left {
    flex-shrink: 0;
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .bar-right {
    flex-shrink: 0;
    display: flex;
    gap: 2px;
  }

  .textarea-wrapper {
    position: relative;
  }

  .input-highlight {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    padding: 4px 2px;
    font: inherit;
    font-size: 13px;
    line-height: normal;
    white-space: pre-wrap;
    word-wrap: break-word;
    color: transparent;
    pointer-events: none;
    overflow: hidden;
  }

  .input-highlight :global(.hl-skill) {
    color: transparent;
    background: rgba(100, 160, 255, 0.12);
    border-radius: 3px;
    border-bottom: 2px solid rgba(100, 160, 255, 0.5);
  }

  .autocomplete-dropdown {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    max-height: 180px;
    overflow-y: auto;
    background: #2a2a2a;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    margin-bottom: 4px;
    z-index: 100;
  }

  .autocomplete-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    border: none;
    background: transparent;
    color: #e0e0e0;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    text-align: left;
  }

  .autocomplete-item.selected {
    background: rgba(100, 160, 255, 0.2);
  }

  .autocomplete-item:hover {
    background: rgba(100, 160, 255, 0.15);
  }

  .autocomplete-name {
    color: rgba(100, 160, 255, 0.9);
    font-family: monospace;
    flex-shrink: 0;
  }

</style>
