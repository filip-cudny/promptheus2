<script lang="ts">
  import { Pencil, Save } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    lineCount,
    tokenCount,
    editMode = $bindable(false),
    saveDisabled = false,
    onsave,
  }: {
    lineCount: number;
    tokenCount?: number;
    editMode?: boolean;
    saveDisabled?: boolean;
    onsave: () => void;
  } = $props();
</script>

<div class="editor-toolbar">
  <span class="line-count">{lineCount} {lineCount === 1 ? "line" : "lines"}{#if tokenCount !== undefined} · ~{tokenCount} tokens{/if}</span>
  <div class="spacer"></div>
  <button
    class="mode-btn"
    class:active={editMode}
    onclick={() => (editMode = !editMode)}
  >
    <Pencil size={ICON_SIZE.sm} />
  </button>
  <button class="save-btn" disabled={saveDisabled} onclick={onsave}>
    <Save size={ICON_SIZE.sm} />
  </button>
</div>

<style>
  .editor-toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    flex-shrink: 0;
  }

  .line-count {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.4);
    user-select: none;
  }

  .spacer {
    flex: 1;
  }

  .mode-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
  }

  .mode-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.8);
  }

  .mode-btn.active {
    background: rgba(74, 158, 187, 0.2);
    color: #7dd3f0;
  }

  .save-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px 8px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
  }

  .save-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.8);
  }

  .save-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }
</style>
