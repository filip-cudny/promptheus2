<script lang="ts">
  import { Copy, Check, History, SquareArrowOutUpRight } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import Chip from "$lib/components/ui/Chip.svelte";
  import { copyHistoryContent } from "$lib/services/history";
  import { openPromptDialog } from "$lib/services/promptDialog";
  import { closeMenu } from "$lib/stores/contextMenu.svelte";
  import { info } from "@tauri-apps/plugin-log";

  interface ChipData {
    content: string;
  }

  interface LastTextEntryRef {
    id: string;
    prompt_id: string | null;
    prompt_name: string | null;
  }

  interface LastInteractionData {
    input: ChipData | null;
    output: ChipData | null;
    transcription: ChipData | null;
    last_text_entry: LastTextEntryRef | null;
  }

  let { data }: { data: LastInteractionData | null } = $props();

  let copyConfirm = $state<string | null>(null);

  async function handleCopy(chipType: string, content: string | undefined | null) {
    if (!content) return;
    await copyHistoryContent(content);
    copyConfirm = chipType;
    setTimeout(() => (copyConfirm = null), 1200);
  }

  async function handleOpenLastInteraction() {
    const entry = data?.last_text_entry;
    if (!entry) return;
    await closeMenu();
    await openPromptDialog(entry.prompt_id ?? "", entry.prompt_name ?? "Chat", entry.id);
  }

  function handleHistory() {
    info("History button clicked — history dialog not yet implemented");
  }

  type ChipEntry = { type: string; label: string; content: string | null };

  let chips = $derived<ChipEntry[]>([
    { type: "input", label: "Input", content: data?.input?.content ?? null },
    { type: "output", label: "Output", content: data?.output?.content ?? null },
    { type: "transcription", label: "Transcription", content: data?.transcription?.content ?? null },
  ]);

  let hasAnyContent = $derived(chips.some((c) => c.content !== null));
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
        onclick={handleHistory}
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
          title={chip.content ?? "No content"}
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
    text-transform: capitalize;
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
    border: none;
    border-radius: 4px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
  }

  .action-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.8);
  }

  .action-btn:disabled {
    color: rgba(255, 255, 255, 0.15);
    cursor: default;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 4px 12px 6px;
  }

  .chip-copy {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .chip-label {
    font-weight: 500;
  }
</style>
