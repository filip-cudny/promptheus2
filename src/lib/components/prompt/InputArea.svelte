<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ContextSection from "./ContextSection.svelte";
  import AttachMenu from "./AttachMenu.svelte";
  import ActionIconButton from "$lib/components/ui/ActionIconButton.svelte";
  import ImageChipBar from "$lib/components/ui/ImageChipBar.svelte";
  import TextChipBar from "$lib/components/ui/TextChipBar.svelte";
  import { SendHorizonal, RefreshCw, Square, CopyCheck } from "lucide-svelte";
  import { getImageFromPasteEvent, extractTextAttachment } from "$lib/utils/paste";
  import { TEXT_ATTACHMENT_CHAR_THRESHOLD } from "$lib/constants/ui";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
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

  let editable: HTMLDivElement | undefined = $state();
  let localText = $state("");
  let localImages = $state<ConversationImage[]>([]);
  let localTextAttachments = $state<string[]>([]);

  let allSkills = $state<SkillSummary[]>([]);
  let showAutocomplete = $state(false);
  let autocompleteItems = $state<SkillSummary[]>([]);
  let autocompleteIndex = $state(0);
  let slashStart = $state(-1);

  let lastSkillPattern = "";

  $effect(() => {
    store.updateInputText(localText);
  });

  $effect(() => {
    store.updateInputImages(localImages);
  });

  $effect(() => {
    store.updateInputTextAttachments(localTextAttachments);
  });

  $effect(() => {
    const storeText = store.inputText;
    if (storeText === "" && localText !== "") {
      localText = "";
      if (editable) {
        editable.innerHTML = "";
      }
    } else if (storeText !== "" && localText === "" && editable) {
      localText = storeText;
      lastSkillPattern = "";
      editable.innerHTML = buildHighlightedHtml(storeText);
      requestAnimationFrame(() => {
        if (editable) {
          restoreCursorOffset(storeText.length);
          editable.focus();
        }
      });
    }
  });

  $effect(() => {
    if (store.inputImages.length === 0 && localImages.length > 0) {
      localImages = [];
    }
  });

  $effect(() => {
    if (store.inputTextAttachments.length === 0 && localTextAttachments.length > 0) {
      localTextAttachments = [];
    }
  });

  let unlistenTextUpdate: (() => void) | null = null;

  onMount(async () => {
    editable?.focus();
    try {
      allSkills = await listSkills();
    } catch {
      allSkills = [];
    }

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
  });

  function getPlainText(): string {
    if (!editable) return "";
    const clone = editable.cloneNode(true) as HTMLElement;
    clone.querySelectorAll("br").forEach((br) => {
      br.replaceWith("\n");
    });
    clone.querySelectorAll("div, p").forEach((block, i) => {
      if (i > 0 || block.previousSibling) {
        block.insertBefore(document.createTextNode("\n"), block.firstChild);
      }
    });
    return (clone.textContent ?? "").replace(/^\n/, "");
  }

  function saveCursorOffset(): number {
    if (!editable) return 0;
    const sel = window.getSelection();
    if (!sel || !sel.rangeCount) return 0;
    const range = sel.getRangeAt(0);
    const pre = document.createRange();
    pre.selectNodeContents(editable);
    pre.setEnd(range.startContainer, range.startOffset);
    return pre.toString().length;
  }

  function restoreCursorOffset(offset: number) {
    if (!editable) return;
    const sel = window.getSelection();
    if (!sel) return;

    let remaining = offset;
    const walker = document.createTreeWalker(editable, NodeFilter.SHOW_TEXT);
    let node: Text | null;

    while ((node = walker.nextNode() as Text | null)) {
      if (remaining <= node.length) {
        const range = document.createRange();
        range.setStart(node, remaining);
        range.collapse(true);
        sel.removeAllRanges();
        sel.addRange(range);
        return;
      }
      remaining -= node.length;
    }

    const range = document.createRange();
    range.selectNodeContents(editable);
    range.collapse(false);
    sel.removeAllRanges();
    sel.addRange(range);
  }

  function escapeHtml(text: string): string {
    return text
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  }

  function buildHighlightedHtml(text: string): string {
    if (!text) return "";
    return text
      .split("\n")
      .map((line) => {
        const match = line.match(/^(\/[a-z0-9-]+)(\s.*)?$/);
        if (match) {
          return `<span class="hl-skill">${escapeHtml(match[1])}</span>${escapeHtml(match[2] ?? "")}`;
        }
        return escapeHtml(line);
      })
      .join("<br>");
  }

  function getSkillPattern(text: string): string {
    const matches: string[] = [];
    for (const line of text.split("\n")) {
      const m = line.match(/^(\/[a-z0-9-]+)/);
      if (m) matches.push(m[1]);
    }
    return matches.join("|");
  }

  function applyHighlighting() {
    if (!editable) return;
    const text = getPlainText();
    const pattern = getSkillPattern(text);

    if (pattern === lastSkillPattern) return;
    lastSkillPattern = pattern;

    const offset = saveCursorOffset();
    editable.innerHTML = buildHighlightedHtml(text);
    restoreCursorOffset(offset);
  }

  function handleEditableInput() {
    const text = getPlainText();
    localText = text;
    applyHighlighting();
    detectSlashCommand();
  }

  function detectSlashCommand() {
    if (!editable) {
      closeAutocomplete();
      return;
    }

    const offset = saveCursorOffset();
    const textBefore = localText.slice(0, offset);
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
    if (!editable || slashStart < 0) return;

    const text = localText;
    const cursorOffset = saveCursorOffset();
    const before = text.slice(0, slashStart);
    const after = text.slice(cursorOffset);
    const newText = `${before}/${skill.name} ${after}`;
    localText = newText;
    lastSkillPattern = "";
    editable.innerHTML = buildHighlightedHtml(newText);

    const newOffset = slashStart + skill.name.length + 2;
    restoreCursorOffset(newOffset);
    closeAutocomplete();
    editable.focus();
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
    if (e.key === "Escape" && showAutocomplete) {
      e.preventDefault();
      closeAutocomplete();
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

  async function handlePaste(e: ClipboardEvent) {
    const textAttachment = extractTextAttachment(e, TEXT_ATTACHMENT_CHAR_THRESHOLD);
    if (textAttachment) {
      localTextAttachments = [...localTextAttachments, textAttachment];
      return;
    }

    const image = await getImageFromPasteEvent(e);
    if (image) {
      localImages = [...localImages, image];
      return;
    }
    const text = e.clipboardData?.getData("text/plain");
    if (text) {
      e.preventDefault();
      document.execCommand("insertText", false, text);
    }
  }
</script>

<div class="input-area">
  {#if contextVisible}
    <ContextSection {store} {contextDisabled} initialCollapsed={contextInitialCollapsed} onHasContent={onContextAutoShow} onClose={onCloseContext} />
  {/if}

  <div class="input-field">
    <TextChipBar bind:textAttachments={localTextAttachments} readonly={false} />
    <ImageChipBar bind:images={localImages} readonly={false} />
    <div class="textarea-wrapper">
      <div
        bind:this={editable}
        class="input-editable"
        contenteditable="true"
        role="textbox"
        tabindex="0"
        aria-multiline="true"
        data-placeholder="Type a message… (use /skill-name for skills)"
        oninput={handleEditableInput}
        onkeydown={handleKeydown}
        onpaste={handlePaste}
      ></div>

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

  .input-editable {
    width: 100%;
    min-height: 1.5em;
    max-height: 40vh;
    overflow-y: auto;
    background: transparent;
    border: none;
    color: #e0e0e0;
    font: inherit;
    font-size: 13px;
    padding: 4px 2px;
    box-sizing: border-box;
    white-space: pre-wrap;
    word-wrap: break-word;
    outline: none;
  }

  .input-editable:empty::before {
    content: attr(data-placeholder);
    color: rgba(255, 255, 255, 0.3);
    pointer-events: none;
  }

  .input-editable :global(.hl-skill) {
    font-weight: 600;
    color: rgba(100, 160, 255, 0.9);
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
