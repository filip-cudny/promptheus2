<script lang="ts">
  import { Check, Copy, Pencil, Save } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";

  let {
    lineCount,
    tokenCount,
    editMode = $bindable(false),
    saveDisabled = false,
    onsave,
    oncopy,
  }: {
    lineCount: number;
    tokenCount?: number;
    editMode?: boolean;
    saveDisabled?: boolean;
    onsave: () => void;
    oncopy?: () => void | Promise<void>;
  } = $props();

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  async function handleCopy() {
    if (!oncopy) return;
    await oncopy();
    copied = true;
    clearTimeout(copyTimer);
    copyTimer = setTimeout(() => (copied = false), 1200);
  }
</script>

<div class="editor-toolbar">
  <span class="line-count">{lineCount} {lineCount === 1 ? "line" : "lines"}{#if tokenCount !== undefined} · ~{tokenCount} tokens{/if}</span>
  <div class="spacer"></div>
  {#if oncopy}
    <span class:active={copied}>
      <ActionIconButton
        icon={copied ? Check : Copy}
        size={ICON_SIZE.sm}
        onclick={handleCopy}
        title={copied ? "Copied" : "Copy"}
      />
    </span>
  {/if}
  <span class:active={editMode}>
    <ActionIconButton
      icon={Pencil}
      size={ICON_SIZE.sm}
      onclick={() => (editMode = !editMode)}
    />
  </span>
  <button class="save-btn" disabled={saveDisabled} onclick={onsave}>
    <Save size={ICON_SIZE.sm} />
  </button>
</div>

<style>
  .editor-toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--border-default);
    flex-shrink: 0;
  }

  .line-count {
    font-size: var(--font-size-sm);
    color: var(--text-disabled);
    user-select: none;
  }

  .spacer {
    flex: 1;
  }

  span.active :global(.action-icon-btn) {
    background: rgba(74, 158, 187, 0.2);
    color: var(--info);
  }

  .save-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .save-btn:hover:not(:disabled) {
    background: var(--surface-overlay);
    color: var(--text-secondary);
  }

  .save-btn:disabled {
    opacity: var(--opacity-disabled);
    cursor: default;
  }
</style>
