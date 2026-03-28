<script lang="ts">
  import { Copy, Check, History } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import { copyHistoryContent } from "$lib/services/history";
  import { info } from "@tauri-apps/plugin-log";

  interface ChipData {
    content: string;
  }

  interface LastInteractionData {
    input: ChipData | null;
    output: ChipData | null;
    transcription: ChipData | null;
  }

  let { data }: { data: LastInteractionData | null } = $props();

  let copyConfirm = $state<string | null>(null);

  function truncateText(text: string, maxLength = 40): string {
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + "\u2026";
  }

  async function handleCopy(chipType: string, content: string | undefined | null) {
    if (!content) return;
    await copyHistoryContent(content);
    copyConfirm = chipType;
    setTimeout(() => (copyConfirm = null), 1200);
  }

  function handleHistory() {
    info("History button clicked — history dialog not yet implemented");
  }

  type Chip = { type: string; label: string; content: string | null };

  let chips = $derived<Chip[]>([
    { type: "input", label: "Input", content: data?.input?.content ?? null },
    { type: "output", label: "Output", content: data?.output?.content ?? null },
    { type: "transcription", label: "Transcription", content: data?.transcription?.content ?? null },
  ]);

  let hasAnyContent = $derived(chips.some((c) => c.content !== null));
</script>

<div class="last-interaction-section">
  <div class="section-header">
    <span class="header-label">Last interaction</span>
    <button
      class="action-btn history-btn"
      onclick={handleHistory}
      title="View execution history"
    >
      <History size={ICON_SIZE.md} />
    </button>
  </div>

  {#if !hasAnyContent}
    <div class="empty-hint">No interactions yet</div>
  {:else}
    <div class="chips">
      {#each chips as chip}
        <button
          class="chip"
          class:chip-disabled={!chip.content}
          disabled={!chip.content}
          title={chip.content ?? "No content"}
          onclick={() => handleCopy(chip.type, chip.content)}
        >
          <span class="chip-copy">
            {#if copyConfirm === chip.type}
              <Check size={ICON_SIZE.md} />
            {:else}
              <Copy size={ICON_SIZE.md} />
            {/if}
          </span>
          <span class="chip-label">{chip.label}</span>
          {#if chip.content}
            <span class="chip-preview">{truncateText(chip.content)}</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .last-interaction-section {
    padding: 2px 0;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 4px 12px;
    color: rgba(255, 255, 255, 0.6);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
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
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
  }

  .action-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.8);
  }

  .empty-hint {
    padding: 4px 12px 6px 12px;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.25);
    font-style: italic;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 4px 12px 6px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    background: #3a3a3a;
    border: 1px solid #555;
    border-radius: 12px;
    font-size: 12px;
    color: #f0f0f0;
    cursor: pointer;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: inherit;
  }

  .chip:hover:not(.chip-disabled) {
    background: #454545;
  }

  .chip-disabled {
    background: #2a2a2a;
    border-color: #444;
    color: #666;
    cursor: default;
  }

  .chip-copy {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .chip-label {
    font-weight: 500;
    flex-shrink: 0;
  }

  .chip-preview {
    color: rgba(255, 255, 255, 0.5);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
