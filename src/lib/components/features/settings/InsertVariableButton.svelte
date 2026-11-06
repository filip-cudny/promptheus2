<script lang="ts">
  import { Braces } from "lucide-svelte";
  import { warn as logWarn } from "@tauri-apps/plugin-log";
  import { ICON_SIZE } from "$lib/constants/ui";
  import FloatingPanel from "$lib/components/shared/ui/FloatingPanel.svelte";
  import { getEnvVars } from "$lib/services/settings";
  import { insertAt } from "$lib/utils/envVarOptions";
  import type { EnvVarStatus } from "$lib/types";

  let {
    field,
    oninsert,
    ariaLabel = "Insert variable",
  }: {
    field: () => HTMLInputElement | HTMLTextAreaElement | null | undefined;
    oninsert: (text: string) => void;
    ariaLabel?: string;
  } = $props();

  let buttonEl = $state<HTMLButtonElement | undefined>();
  let open = $state(false);
  let vars = $state<EnvVarStatus[]>([]);

  async function toggle() {
    open = !open;
    if (!open) return;
    try {
      vars = await getEnvVars();
    } catch (e) {
      logWarn("Failed to read get_env_vars: " + e);
    }
  }

  function insert(name: string) {
    open = false;
    const el = field();
    if (!el) return;
    const start = el.selectionStart ?? el.value.length;
    const end = el.selectionEnd ?? el.value.length;
    const { text, caret } = insertAt(el.value, start, end, `\${${name}}`);
    el.value = text;
    oninsert(text);
    el.focus();
    el.setSelectionRange(caret, caret);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key !== "Escape" || !open) return;
    e.stopPropagation();
    open = false;
    buttonEl?.focus();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<button
  bind:this={buttonEl}
  type="button"
  class="editor-icon-btn"
  class:active={open}
  aria-label={ariaLabel}
  title={ariaLabel}
  onclick={toggle}
>
  <Braces size={ICON_SIZE.md} />
</button>

<FloatingPanel visible={open} anchorEl={buttonEl} position="below" fitContent onclose={() => (open = false)}>
  <ul class="list" role="listbox" aria-label={ariaLabel}>
    {#each vars as v (v.name)}
      <li>
        <button type="button" class="option" onclick={() => insert(v.name)}>
          <code>{v.name}</code>
          {#if !v.set}<span class="missing">missing</span>{/if}
        </button>
      </li>
    {:else}
      <li class="empty">No variables listed.</li>
    {/each}
  </ul>
</FloatingPanel>

<style>
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    min-width: 200px;
    max-width: 320px;
  }

  .option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    width: 100%;
    padding: var(--space-2) var(--space-5);
    background: transparent;
    border: none;
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
  }

  .option:hover,
  .option:focus-visible {
    background: var(--surface-overlay-faint);
    outline: none;
  }

  code {
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
  }

  .missing {
    font-size: var(--font-size-xs);
    color: var(--text-muted);
  }

  .empty {
    padding: var(--space-2) var(--space-5);
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }
</style>
