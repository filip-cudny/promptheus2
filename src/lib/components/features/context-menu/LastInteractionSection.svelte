<script lang="ts">
  import {
    Copy,
    Check,
    History,
    SquareArrowOutUpRight,
    CircleAlert,
    RotateCcw,
    LoaderCircle,
  } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import Chip from "$lib/components/shared/ui/Chip.svelte";
  import type {
    LastInteractionData,
    LastTextEntryRef,
  } from "./itemExtractors";

  let {
    data,
    onCopyContent,
    onOpenLastInteraction,
    onOpenHistory,
    onRetryTranscription,
  }: {
    data: LastInteractionData | null;
    onCopyContent: (content: string) => Promise<void>;
    onOpenLastInteraction: (entry: LastTextEntryRef) => Promise<void>;
    onOpenHistory: () => Promise<void>;
    onRetryTranscription: (entryId: string) => Promise<void>;
  } = $props();

  let copyConfirm = $state<string | null>(null);

  async function handleCopy(chipType: string, content: string | undefined | null) {
    if (!content) return;
    await onCopyContent(content);
    copyConfirm = chipType;
    setTimeout(() => (copyConfirm = null), 1200);
  }

  async function handleOpenLastInteraction() {
    const entry = data?.last_text_entry;
    if (!entry) return;
    await onOpenLastInteraction(entry);
  }

  type ChipEntry = { type: string; label: string; content: string | null; preview: string | null };

  let transcription = $derived(data?.transcription ?? null);
  let transcriptionFailed = $derived(transcription?.status === "failed");
  let transcriptionRetrying = $derived(transcription?.status === "retrying");

  let chips = $derived<ChipEntry[]>([
    { type: "input", label: "Input", content: data?.input?.content ?? null, preview: data?.input?.preview ?? null },
    { type: "output", label: "Output", content: data?.output?.content ?? null, preview: data?.output?.preview ?? null },
  ]);

  let transcriptionTitle = $derived.by(() => {
    if (transcriptionRetrying) return "Retrying transcription…";
    if (transcriptionFailed) {
      const reason = transcription?.error ?? "Transcription failed";
      return transcription?.has_audio
        ? `${reason} — audio kept, retry available`
        : `${reason} — audio no longer available`;
    }
    return transcription?.preview ?? "No content";
  });

  let hasAnyContent = $derived(
    chips.some((c) => c.content !== null) || transcription !== null,
  );

  async function handleRetry() {
    if (!transcription?.entry_id || !transcription.has_audio) return;
    await onRetryTranscription(transcription.entry_id);
  }
</script>

<div class="last-interaction-section">
  <div class="section-header">
    <span class="header-label">Last interaction</span>
    <div class="header-actions">
      <button
        class="action-btn"
        onclick={handleOpenLastInteraction}
        disabled={!data?.last_text_entry}
        title="Open last interaction"
      >
        <SquareArrowOutUpRight size={ICON_SIZE.md} />
      </button>
      <button
        class="action-btn"
        onclick={onOpenHistory}
        title="View execution history"
      >
        <History size={ICON_SIZE.md} />
      </button>
    </div>
  </div>

  {#if hasAnyContent}
    <div class="chips">
      {#each chips as chip}
        <Chip
          onclick={() => handleCopy(chip.type, chip.content)}
          disabled={!chip.content}
          title={chip.preview ?? "No content"}
        >
          <span class="chip-copy">
            {#if copyConfirm === chip.type}
              <Check size={ICON_SIZE.md} />
            {:else}
              <Copy size={ICON_SIZE.md} />
            {/if}
          </span>
          <span class="chip-label">{chip.label}</span>
        </Chip>
      {/each}

      <div class="transcription-group" class:failed={transcriptionFailed}>
        <Chip
          onclick={() => handleCopy("transcription", transcription?.content)}
          disabled={!transcription?.content || transcriptionRetrying}
          title={transcriptionTitle}
        >
          <span class="chip-copy">
            {#if transcriptionRetrying}
              <span class="spinning"><LoaderCircle size={ICON_SIZE.md} /></span>
            {:else if transcriptionFailed}
              <CircleAlert size={ICON_SIZE.md} />
            {:else if copyConfirm === "transcription"}
              <Check size={ICON_SIZE.md} />
            {:else}
              <Copy size={ICON_SIZE.md} />
            {/if}
          </span>
          <span class="chip-label">
            {transcriptionRetrying ? "Retrying…" : "Transcription"}
          </span>
        </Chip>

        {#if transcriptionFailed && transcription?.has_audio}
          <button
            class="retry-btn"
            onclick={handleRetry}
            title="Retry transcription — result goes to the clipboard"
            aria-label="Retry transcription"
          >
            <RotateCcw size={ICON_SIZE.md} />
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .last-interaction-section {
    padding: var(--space-1) var(--space-0);
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--space-2) var(--space-6);
    color: var(--text-muted);
    font-size: var(--font-size-sm);
    text-transform: capitalize;
    letter-spacing: var(--tracking-label);
    box-sizing: border-box;
  }

  .header-label {
    display: flex;
    align-items: center;
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 3px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .action-btn:hover:not(:disabled) {
    background: var(--surface-overlay);
    color: var(--text-secondary);
  }

  .action-btn:disabled {
    color: rgba(255, 255, 255, 0.15);
    cursor: default;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-6) var(--space-3);
  }

  .chip-copy {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .chip-label {
    font-weight: var(--font-weight-medium);
  }

  .transcription-group {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .transcription-group.failed :global(.chip) {
    color: var(--danger);
    border-color: var(--danger-border);
  }

  .retry-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 3px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--danger);
    cursor: pointer;
  }

  .retry-btn:hover {
    background: var(--surface-overlay);
  }

  .spinning {
    display: flex;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
