<script lang="ts">
  import { onMount } from "svelte";
  import { Braces, Check, AlertCircle } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import {
    getPrompt,
    savePrompt,
    type PromptDoc,
    type PromptKind,
  } from "$lib/services/prompts";
  import EnvPlaceholdersPopover from "./EnvPlaceholdersPopover.svelte";

  const AUTOSAVE_DEBOUNCE_MS = 800;
  const SAVED_BADGE_TTL_MS = 2500;

  let {
    kind,
    description,
  }: {
    kind: PromptKind;
    description?: string;
  } = $props();

  let doc = $state<PromptDoc | null>(null);
  let content = $state("");
  let savedContent = $state("");
  let textarea = $state<HTMLTextAreaElement | undefined>(undefined);
  let highlightLayer = $state<HTMLPreElement | undefined>(undefined);
  let placeholdersBtn = $state<HTMLButtonElement | undefined>(undefined);
  let placeholdersOpen = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let lastSavedAt = $state<number | null>(null);
  let now = $state(Date.now());
  let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  let nowTimer: ReturnType<typeof setInterval> | null = null;

  const PLACEHOLDER_RE = /\{\{[^{}\n]+\}\}/g;

  const highlighted = $derived(buildHighlighted(content, doc?.supports_env_placeholders ?? false));

  function escapeHtml(s: string): string {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  }

  function buildHighlighted(text: string, decorate: boolean): string {
    const padded = text.endsWith("\n") ? text + " " : text;
    if (!decorate) return escapeHtml(padded);
    let out = "";
    let lastIndex = 0;
    PLACEHOLDER_RE.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = PLACEHOLDER_RE.exec(padded)) !== null) {
      out += escapeHtml(padded.slice(lastIndex, m.index));
      out += `<span class="placeholder-token">${escapeHtml(m[0])}</span>`;
      lastIndex = m.index + m[0].length;
    }
    out += escapeHtml(padded.slice(lastIndex));
    return out;
  }

  function syncScroll() {
    if (!textarea || !highlightLayer) return;
    highlightLayer.scrollTop = textarea.scrollTop;
    highlightLayer.scrollLeft = textarea.scrollLeft;
  }

  let dirty = $derived(content !== savedContent);
  let recentlySaved = $derived(
    !dirty && lastSavedAt !== null && now - lastSavedAt < SAVED_BADGE_TTL_MS,
  );

  onMount(() => {
    void load();
    nowTimer = setInterval(() => {
      now = Date.now();
    }, 500);
    return () => {
      if (nowTimer) clearInterval(nowTimer);
      if (autosaveTimer) clearTimeout(autosaveTimer);
    };
  });

  async function load() {
    try {
      const fetched = await getPrompt(kind);
      doc = fetched;
      content = fetched.content;
      savedContent = fetched.content;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  $effect(() => {
    if (!doc) return;
    const _track = content;
    if (autosaveTimer) clearTimeout(autosaveTimer);
    if (!dirty) return;
    autosaveTimer = setTimeout(() => {
      void save();
    }, AUTOSAVE_DEBOUNCE_MS);
  });

  async function save() {
    if (!doc) return;
    if (saving) return;
    if (content === savedContent) return;
    saving = true;
    error = null;
    const snapshot = content;
    try {
      await savePrompt(kind, snapshot);
      savedContent = snapshot;
      lastSavedAt = Date.now();
      now = Date.now();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      saving = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      void save();
    }
  }

  function insertAtCursor(token: string) {
    if (!textarea) return;
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const next = content.slice(0, start) + token + content.slice(end);
    content = next;
    queueMicrotask(() => {
      if (!textarea) return;
      const pos = start + token.length;
      textarea.focus();
      textarea.setSelectionRange(pos, pos);
    });
  }

  function dotTooltip(): string {
    if (error) return `Failed to save: ${error}`;
    if (saving) return "Saving…";
    if (dirty) return "Unsaved — autosaving (⌘S to flush)";
    if (lastSavedAt) return `Saved ${formatAgo(now - lastSavedAt)} ago — autosaves on edit`;
    return "Autosaves on edit (⌘S to flush)";
  }

  function formatAgo(ms: number): string {
    const s = Math.max(0, Math.floor(ms / 1000));
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60);
    if (m < 60) return `${m}m`;
    const h = Math.floor(m / 60);
    return `${h}h`;
  }
</script>

<section class="prompt-editor">
  <header class="head">
    <div class="title-row">
      <h3 class="title">{doc?.label ?? "…"}</h3>
      <span
        class="status-dot"
        class:dirty
        class:saving
        class:saved={recentlySaved}
        class:err={error}
        title={dotTooltip()}
        aria-label={dotTooltip()}
      ></span>
      {#if recentlySaved && lastSavedAt !== null}
        <span class="saved-stamp" aria-hidden="true">
          <Check size={10} /> saved
        </span>
      {/if}
      {#if error}
        <span class="err-stamp" title={error}>
          <AlertCircle size={10} /> save failed
        </span>
      {/if}

      <div class="head-actions">
        {#if doc?.supports_env_placeholders}
          <button
            type="button"
            class="icon-btn"
            class:active={placeholdersOpen}
            bind:this={placeholdersBtn}
            onclick={() => (placeholdersOpen = !placeholdersOpen)}
            title="Insert placeholder"
            aria-label="Insert placeholder"
          >
            <Braces size={ICON_SIZE.sm} />
          </button>
        {/if}
        {#if doc?.path}
          <details class="path-details">
            <summary title="Show file path">…</summary>
            <code class="path">{doc.path}</code>
          </details>
        {/if}
      </div>
    </div>
    {#if description}
      <p class="description">{description}</p>
    {/if}
  </header>

  <div class="editor-pane">
    <div class="editor-frame" class:disabled={!doc}>
      <pre
        class="highlight-layer"
        bind:this={highlightLayer}
        aria-hidden="true">{@html highlighted}</pre>
      <textarea
        bind:this={textarea}
        bind:value={content}
        onkeydown={handleKeydown}
        onscroll={syncScroll}
        spellcheck="false"
        rows="14"
        placeholder={doc ? "" : "Loading…"}
        disabled={!doc}
      ></textarea>
    </div>
  </div>

  {#if doc?.supports_env_placeholders}
    <EnvPlaceholdersPopover
      visible={placeholdersOpen}
      anchorEl={placeholdersBtn}
      onclose={() => (placeholdersOpen = false)}
      onInsert={(token) => {
        insertAtCursor(token);
        placeholdersOpen = false;
      }}
    />
  {/if}
</section>

<style>
  .prompt-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-height: 0;
    flex: 1;
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .title {
    margin: 0;
    font-size: var(--font-size-base);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-faint);
    transition: background var(--motion-fast) var(--ease-default),
      transform var(--motion-fast) var(--ease-default);
    cursor: help;
  }

  .status-dot.dirty {
    background: var(--accent);
  }

  .status-dot.saving {
    background: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }

  .status-dot.saved {
    background: var(--success);
  }

  .status-dot.err {
    background: var(--danger);
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.55; }
    50% { opacity: 1; }
  }

  .saved-stamp,
  .err-stamp {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: var(--font-size-xs);
    color: var(--text-muted);
    animation: fade-in var(--motion-default) var(--ease-default);
  }

  .saved-stamp {
    color: var(--success);
  }

  .err-stamp {
    color: var(--danger);
  }

  @keyframes fade-in {
    from { opacity: 0; transform: translateY(-2px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .head-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    transition: background var(--motion-fast) var(--ease-default),
      color var(--motion-fast) var(--ease-default),
      border-color var(--motion-fast) var(--ease-default);
  }

  .icon-btn:hover {
    background: var(--surface-overlay-faint);
    color: var(--text-primary);
  }

  .icon-btn.active {
    background: var(--accent-bg-soft);
    color: var(--accent);
    border-color: var(--accent-border);
  }

  .path-details {
    position: relative;
    font-size: var(--font-size-xs);
  }

  .path-details summary {
    list-style: none;
    cursor: pointer;
    color: var(--text-faint);
    padding: 0 var(--space-2);
    border-radius: var(--radius-sm);
    user-select: none;
  }

  .path-details summary::-webkit-details-marker {
    display: none;
  }

  .path-details summary:hover {
    color: var(--text-muted);
    background: var(--surface-overlay-faint);
  }

  .path-details[open] summary {
    color: var(--text-muted);
  }

  .path-details code {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: var(--space-1);
    padding: var(--space-2) var(--space-3);
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    color: var(--text-muted);
    background: var(--surface-floating-popover);
    border: 1px solid var(--surface-floating-popover-border);
    border-radius: var(--radius-sm);
    white-space: nowrap;
    box-shadow: var(--shadow-md);
    z-index: var(--z-overlay);
  }

  .description {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .editor-pane {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .editor-frame {
    position: relative;
    flex: 1;
    min-height: 320px;
    background: var(--surface-elevated);
    border: 1px solid var(--border-faint);
    border-radius: var(--radius-md);
    overflow: hidden;
    transition: border-color var(--motion-fast) var(--ease-default);
  }

  .editor-frame:focus-within {
    border-color: var(--accent-border);
  }

  .editor-frame.disabled {
    opacity: var(--opacity-disabled);
  }

  .highlight-layer,
  .editor-frame textarea {
    position: absolute;
    inset: 0;
    margin: 0;
    padding: var(--space-4);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    line-height: 1.6;
    white-space: pre-wrap;
    overflow-wrap: break-word;
    word-break: break-word;
    tab-size: 2;
    border: none;
    background: transparent;
    box-sizing: border-box;
  }

  .highlight-layer {
    pointer-events: none;
    color: var(--text-primary);
    overflow: auto;
    scrollbar-width: none;
  }

  .highlight-layer::-webkit-scrollbar {
    display: none;
  }

  .editor-frame textarea {
    color: transparent;
    caret-color: var(--text-primary);
    resize: none;
    outline: none;
    overflow: auto;
  }

  .editor-frame textarea::selection {
    background: var(--accent-bg);
    color: transparent;
  }

  .highlight-layer :global(.placeholder-token) {
    color: var(--accent);
    background: var(--accent-bg-soft);
    border-radius: 2px;
    box-shadow: 0 0 0 1px var(--accent-bg-soft);
    -webkit-box-decoration-break: clone;
    box-decoration-break: clone;
  }
</style>
