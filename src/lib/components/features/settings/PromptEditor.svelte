<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Braces } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import {
    getPrompt,
    savePrompt,
    type PromptDoc,
    type PromptKind,
  } from "$lib/services/prompts";
  import { useSaveTracker } from "$lib/stores/saveTracker.svelte";
  import { getSettingsStore } from "$lib/stores/settings.svelte";
  import SaveStatusIndicator from "$lib/components/shared/widgets/SaveStatusIndicator.svelte";
  import HighlightedTextarea from "$lib/components/shared/ui/HighlightedTextarea.svelte";
  import FilePathReveal from "$lib/components/shared/ui/FilePathReveal.svelte";
  import EnvPlaceholdersPopover from "./EnvPlaceholdersPopover.svelte";

  let {
    kind,
    description,
  }: {
    kind: PromptKind;
    description?: string;
  } = $props();

  const settingsStore = getSettingsStore();

  let doc = $state<PromptDoc | null>(null);
  let content = $state("");
  let savedContent = $state("");
  let textarea = $state<ReturnType<typeof HighlightedTextarea> | undefined>(undefined);
  let placeholdersBtn = $state<HTMLButtonElement | undefined>(undefined);
  let placeholdersOpen = $state(false);

  const tracker = useSaveTracker({
    debounceMs: settingsStore.settings?.autosave_debounce_ms ?? 1000,
  });

  const PLACEHOLDER_RE = /\{\{[^{}\n]+\}\}/g;

  const highlightPattern = $derived(
    doc?.supports_env_placeholders ? PLACEHOLDER_RE : undefined,
  );

  onMount(() => {
    void load();
    tracker.attachKeyboard(window);
    tracker.attachBeforeUnload(window);
  });

  onDestroy(() => {
    tracker.destroy();
  });

  async function load() {
    try {
      const fetched = await getPrompt(kind);
      doc = fetched;
      content = fetched.content;
      savedContent = fetched.content;
    } catch (e) {
      tracker.setError(e instanceof Error ? e.message : String(e));
    }
  }

  $effect(() => {
    if (!doc) return;
    const _track = content;
    if (content === savedContent) {
      tracker.cancel();
      return;
    }
    tracker.scheduleSave(persist);
  });

  async function persist() {
    if (!doc) return;
    if (content === savedContent) return;
    const snapshot = content;
    await savePrompt(kind, snapshot);
    savedContent = snapshot;
  }
</script>

<section class="prompt-editor">
  <header class="head">
    <div class="title-row">
      <h3 class="title">{doc?.label ?? "…"}</h3>
      <SaveStatusIndicator {tracker} />

      <div class="head-actions">
        {#if doc?.supports_env_placeholders}
          <button
            type="button"
            class="editor-icon-btn"
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
          <FilePathReveal path={doc.path} />
        {/if}
      </div>
    </div>
    {#if description}
      <p class="description">{description}</p>
    {/if}
  </header>

  <div class="editor-pane">
    <HighlightedTextarea
      bind:this={textarea}
      bind:value={content}
      {highlightPattern}
      placeholder={doc ? "" : "Loading…"}
      disabled={!doc}
    />
  </div>

  {#if doc?.supports_env_placeholders}
    <EnvPlaceholdersPopover
      visible={placeholdersOpen}
      anchorEl={placeholdersBtn}
      onclose={() => (placeholdersOpen = false)}
      onInsert={(token) => {
        textarea?.insert(token);
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

  .head-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--space-1);
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
</style>
