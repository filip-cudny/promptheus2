<script lang="ts">
  import { ArrowUp, X } from "lucide-svelte";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";
  import type { ComposerDictation } from "./useComposerDictation.svelte";

  let { dictation }: { dictation: ComposerDictation } = $props();

  const phase = $derived(dictation.state);

  function formatElapsed(ms: number): string {
    const seconds = Math.floor(ms / 1000);
    return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
  }
</script>

<div class="dictation-bar">
  {#if phase.kind === "recording"}
    <div class="meter" aria-hidden="true">
      <div class="meter-fill" style:width="{Math.round(phase.level * 100)}%"></div>
    </div>
    <span class="time">{formatElapsed(phase.elapsedMs)}</span>
    <span class="spacer"></span>
    <ActionIconButton icon={X} label="Cancel" onclick={() => dictation.cancel()} />
    <button class="finish" title="Finish" onclick={() => dictation.finish()}>
      <ArrowUp size={14} strokeWidth={3.4} />
    </button>
  {:else if phase.kind === "transcribing"}
    <span class="status">Transcribing…</span>
    <span class="spacer"></span>
    <ActionIconButton icon={X} label="Cancel" onclick={() => dictation.cancel()} />
  {:else if phase.kind === "error"}
    <span class="status error" title={phase.message}>{phase.message}</span>
    <span class="spacer"></span>
    {#if phase.retryable}
      <button class="text-btn" onclick={() => dictation.retry()}>Retry</button>
    {/if}
    <button class="text-btn" onclick={() => dictation.dismiss()}>Dismiss</button>
  {/if}
</div>

<style>
  .dictation-bar {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .meter {
    width: 64px;
    height: 4px;
    border-radius: 2px;
    background: var(--surface-overlay);
    overflow: hidden;
  }

  .meter-fill {
    height: 100%;
    background: var(--text-primary);
    transition: width 50ms linear;
  }

  .time,
  .status {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    user-select: none;
  }

  .status {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status.error {
    color: var(--danger);
  }

  .spacer {
    flex: 1;
  }

  .finish {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border: none;
    border-radius: 50%;
    background: var(--text-primary);
    color: var(--surface-base);
    cursor: pointer;
  }

  .text-btn {
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-size-sm);
    padding: var(--space-1) var(--space-3);
    cursor: pointer;
  }

  .text-btn:hover {
    background: var(--surface-overlay);
    color: var(--text-primary);
  }
</style>
