<script lang="ts">
  import { getSkillsStore } from "$lib/stores/skills.svelte";
  import { highlightSkills, fuzzyMatch } from "$lib/utils/skillHighlight";
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

  let textarea: HTMLTextAreaElement | undefined = $state();
  let overlay: HTMLDivElement | undefined = $state();
  const skillsStore = getSkillsStore();
  let showAutocomplete = $state(false);
  let autocompleteItems = $state<SkillSummary[]>([]);
  let autocompleteIndex = $state(0);
  let slashStart = $state(-1);
  let dropdownEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (showAutocomplete && dropdownEl) {
      const _idx = autocompleteIndex;
      const selected = dropdownEl.querySelector(".autocomplete-item.selected");
      selected?.scrollIntoView({ block: "nearest" });
    }
  });

  const highlightedHtml = $derived(
    highlightSkills(text, classifySkillToken, "<br>"),
  );

  export function focus() {
    textarea?.focus();
  }

  export function getElement(): HTMLTextAreaElement | undefined {
    return textarea;
  }

  export function setTextAndHighlight(newText: string) {
    text = newText;
  }

  export function restoreCursor(offset: number) {
    if (!textarea) return;
    textarea.selectionStart = textarea.selectionEnd = offset;
  }

  function classifySkillToken(token: string, finished: boolean): string | null {
    const name = token.slice(1);
    if (!name) return "hl-skill-partial";
    if (skillsStore.nameSet.has(name)) return "hl-skill";
    if (finished) return null;
    const hasMatch = skillsStore.items.some(
      (s) =>
        fuzzyMatch(name, s.name) !== null ||
        fuzzyMatch(name, s.display_name.toLowerCase()) !== null,
    );
    return hasMatch ? "hl-skill-partial" : null;
  }

  function handleInput() {
    if (textarea) text = textarea.value;
    detectSlashCommand();
    oninput?.();
  }

  function syncScroll() {
    if (overlay && textarea) {
      overlay.scrollTop = textarea.scrollTop;
    }
  }

  function detectSlashCommand() {
    if (!textarea) {
      closeAutocomplete();
      return;
    }

    const offset = textarea.selectionStart;
    const textBefore = text.slice(0, offset);
    const match = textBefore.match(/(^|\s)(\/[a-z0-9-]*)$/);

    if (match) {
      const slashToken = match[2];
      const query = slashToken.slice(1);
      slashStart = offset - slashToken.length;

      const scored = skillsStore.items
        .map((s) => {
          if (!query) return { skill: s, score: 0 };
          const nameScore = fuzzyMatch(query, s.name);
          const displayScore = fuzzyMatch(query, s.display_name.toLowerCase());
          const best = Math.max(nameScore ?? -1, displayScore ?? -1);
          return best >= 0 ? { skill: s, score: best } : null;
        })
        .filter((x): x is { skill: SkillSummary; score: number } => x !== null)
        .sort((a, b) => b.score - a.score);

      if (scored.length > 0) {
        autocompleteItems = scored.map((s) => s.skill);
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

    const cursorOffset = textarea.selectionStart;
    const before = text.slice(0, slashStart);
    const after = text.slice(cursorOffset);
    text = `${before}/${skill.name} ${after}`;

    const newOffset = slashStart + skill.name.length + 2;
    closeAutocomplete();
    requestAnimationFrame(() => {
      if (!textarea) return;
      textarea.value = text;
      textarea.selectionStart = textarea.selectionEnd = newOffset;
      textarea.focus();
    });
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
    bind:this={overlay}
    class="highlight-overlay {editableClass}"
    aria-hidden="true"
  >{@html highlightedHtml}&nbsp;</div>
  <textarea
    bind:this={textarea}
    bind:value={text}
    class="skill-textarea {editableClass}"
    {placeholder}
    rows={1}
    oninput={handleInput}
    onkeydown={handleKeydown}
    onpaste={onpaste}
    onscroll={syncScroll}
  ></textarea>

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

  .highlight-overlay,
  .skill-textarea {
    width: 100%;
    min-height: 1.5em;
    background: transparent;
    border: none;
    color: #e0e0e0;
    font: inherit;
    white-space: pre-wrap;
    word-wrap: break-word;
    overflow-y: auto;
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    line-height: inherit;
    letter-spacing: inherit;
  }

  .skill-textarea {
    position: relative;
    z-index: 1;
    color: transparent;
    caret-color: #e0e0e0;
    outline: none;
    resize: none;
  }

  .highlight-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 0;
    overflow: hidden;
  }

  .highlight-overlay :global(.hl-skill) {
    color: rgba(100, 160, 255, 0.9);
  }

  .highlight-overlay :global(.hl-skill-partial) {
    color: rgba(100, 160, 255, 0.6);
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
