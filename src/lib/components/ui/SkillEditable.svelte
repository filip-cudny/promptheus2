<script lang="ts">
  import { onMount } from "svelte";
  import { getSkillsStore } from "$lib/stores/skills.svelte";
  import { highlightSkills } from "$lib/utils/skillHighlight";
  import type { SkillSummary } from "$lib/types";

  let {
    text = $bindable(""),
    placeholder = "",
    editableClass = "",
    oninput,
    onkeydown,
    onpaste,
  }: {
    text?: string;
    placeholder?: string;
    editableClass?: string;
    oninput?: () => void;
    onkeydown?: (e: KeyboardEvent) => void;
    onpaste?: (e: ClipboardEvent) => void;
  } = $props();

  let editable: HTMLDivElement | undefined = $state();
  const skillsStore = getSkillsStore();
  let showAutocomplete = $state(false);
  let autocompleteItems = $state<SkillSummary[]>([]);
  let autocompleteIndex = $state(0);
  let slashStart = $state(-1);
  let lastSkillPattern = "";
  let dropdownEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (showAutocomplete && dropdownEl) {
      const _idx = autocompleteIndex;
      const selected = dropdownEl.querySelector(".autocomplete-item.selected");
      selected?.scrollIntoView({ block: "nearest" });
    }
  });

  onMount(() => {
    if (text && editable) {
      const offset = saveCursorOffset();
      lastSkillPattern = "";
      editable.innerHTML = buildHighlightedHtml(text);
      restoreCursorOffset(offset);
    }
  });

  export function focus() {
    editable?.focus();
  }

  export function getElement(): HTMLDivElement | undefined {
    return editable;
  }

  export function setTextAndHighlight(newText: string) {
    text = newText;
    lastSkillPattern = "";
    if (editable) {
      editable.innerHTML = buildHighlightedHtml(newText);
    }
  }

  export function restoreCursor(offset: number) {
    restoreCursorOffset(offset);
  }

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

  function isKnownSkill(token: string): boolean {
    return skillsStore.nameSet.has(token.slice(1));
  }

  function buildHighlightedHtml(t: string): string {
    return highlightSkills(t, isKnownSkill, "hl-skill", "<br>");
  }

  function getSkillPattern(t: string): string {
    const matches: string[] = [];
    for (const line of t.split("\n")) {
      for (const m of line.matchAll(/(^|\s)(\/[a-z0-9-]+)(\s|$)/g)) {
        if (isKnownSkill(m[2])) matches.push(m[2]);
      }
    }
    return matches.join("|");
  }

  function hasBrowserStyledSpans(): boolean {
    if (!editable) return false;
    return editable.querySelector("span:not(.hl-skill)") !== null;
  }

  function applyHighlighting() {
    if (!editable) return;
    const t = getPlainText();
    const pattern = getSkillPattern(t);
    if (pattern === lastSkillPattern && !hasBrowserStyledSpans()) return;
    lastSkillPattern = pattern;
    const offset = saveCursorOffset();
    editable.innerHTML = buildHighlightedHtml(t);
    restoreCursorOffset(offset);
  }

  function handleInput() {
    const t = getPlainText();
    text = t;

    if (!t && editable) {
      editable.innerHTML = "";
      lastSkillPattern = "";
    } else {
      applyHighlighting();
    }

    detectSlashCommand();
    oninput?.();
  }

  function detectSlashCommand() {
    if (!editable) {
      closeAutocomplete();
      return;
    }

    const offset = saveCursorOffset();
    const textBefore = text.slice(0, offset);
    const match = textBefore.match(/(^|\s)(\/[a-z0-9-]*)$/);

    if (match) {
      const slashToken = match[2];
      const query = slashToken.slice(1);
      slashStart = offset - slashToken.length;
      const filtered = skillsStore.items.filter(
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

    const cursorOffset = saveCursorOffset();
    const before = text.slice(0, slashStart);
    const after = text.slice(cursorOffset);
    const newText = `${before}/${skill.name} ${after}`;
    text = newText;
    lastSkillPattern = "";
    editable.innerHTML = buildHighlightedHtml(newText);

    const newOffset = slashStart + skill.name.length + 2;
    restoreCursorOffset(newOffset);
    closeAutocomplete();
    editable.focus();
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

    onkeydown?.(e);
  }
</script>

<div class="skill-editable-wrapper">
  <div
    bind:this={editable}
    class="skill-editable {editableClass}"
    contenteditable="true"
    role="textbox"
    tabindex="0"
    aria-multiline="true"
    data-placeholder={placeholder}
    oninput={handleInput}
    onkeydown={handleKeydown}
    onpaste={onpaste}
  ></div>

  {#if showAutocomplete && autocompleteItems.length > 0}
    <div class="autocomplete-dropdown" bind:this={dropdownEl}>
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

<style>
  .skill-editable-wrapper {
    position: relative;
  }

  .skill-editable {
    width: 100%;
    min-height: 1.5em;
    overflow-y: auto;
    background: transparent;
    border: none;
    color: #e0e0e0;
    font: inherit;
    white-space: pre-wrap;
    word-wrap: break-word;
    outline: none;
    box-sizing: border-box;
  }

  .skill-editable:empty::before {
    content: attr(data-placeholder);
    color: rgba(255, 255, 255, 0.3);
    pointer-events: none;
  }

  .skill-editable :global(.hl-skill) {
    font-weight: 600;
    color: rgba(100, 160, 255, 0.9);
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
