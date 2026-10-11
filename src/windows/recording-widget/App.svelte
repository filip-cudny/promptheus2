<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { attachConsole, error as logError } from "@tauri-apps/plugin-log";
  import {
    Check,
    ChevronDown,
    ChevronUp,
    ClipboardPaste,
    Image as ImageIcon,
    MessageSquareShare,
    Pause,
    Play,
    Square,
    Type,
    X,
  } from "lucide-svelte";

  type WidgetState = "recording" | "paused" | "processing" | "done";

  interface WidgetPayload {
    state: WidgetState;
    level: number;
    elapsedMs: number;
    monochromatic: boolean;
  }

  interface PillTarget {
    id: string;
    label: string;
  }

  interface PillItem {
    index: number;
    kind: "text" | "image";
    label: string;
  }

  interface PillModel {
    targets: PillTarget[];
    selectedId: string;
    selectedLabel: string;
    targetLocked: boolean;
    items: PillItem[];
    contextVersion: number;
    dropdownUp: boolean;
  }

  const PILL_WIDTH = 240;
  const PILL_HEIGHT = 80;
  const ITEMS_ROW_HEIGHT = 32;
  const ROW_HEIGHT = PILL_HEIGHT / 2;
  const RADIUS = 16;
  const PADDING_X = 14;
  const ITEM_GAP = 4;
  const ITEM_CHAR_WIDTH = 7;
  const ITEM_CHIP_BASE_WIDTH = 28;
  const ITEM_REMOVE_WIDTH = 16;
  const ITEM_OVERFLOW_WIDTH = 32;
  const DROPDOWN_ROW_HEIGHT = 28;
  const DROPDOWN_PADDING = 4;
  const DEFAULT_TARGET_LABEL = "Dictation";
  const BAR_FACTORS = [0.55, 0.8, 0.65, 1, 0.75, 0.95, 0.6, 0.85, 0.5];
  const MIN_BAR = 0.12;
  const DONE_COLOR = "#62A878";
  const WAVE_BAR_COUNT = 35;
  const WAVE_PERIOD_MS = 1200;
  const WAVE_BARS = Array.from({ length: WAVE_BAR_COUNT }, (_, i) => (i / WAVE_BAR_COUNT - 1) * WAVE_PERIOD_MS);

  const isMac = /Mac/.test(navigator.platform);

  let widgetState = $state<WidgetState>("recording");
  let level = $state(0);
  let elapsedMs = $state(0);
  let monochromatic = $state(true);
  let model = $state<PillModel>({
    targets: [],
    selectedId: "",
    selectedLabel: DEFAULT_TARGET_LABEL,
    targetLocked: false,
    items: [],
    contextVersion: 0,
    dropdownUp: false,
  });
  let dropdownOpen = $state(false);
  let listUp = $state(false);
  let unlisteners: UnlistenFn[] = [];

  const clock = $derived.by(() => {
    const totalSeconds = Math.floor(elapsedMs / 1000);
    const seconds = String(totalSeconds % 60).padStart(2, "0");
    return `${Math.floor(totalSeconds / 60)}:${seconds}`;
  });

  const interactive = $derived(!model.targetLocked && widgetState !== "done");
  const hasItems = $derived(model.items.length > 0);
  const pillHeight = $derived(PILL_HEIGHT + (hasItems ? ITEMS_ROW_HEIGHT : 0));
  const dropdownHeight = $derived(model.targets.length * DROPDOWN_ROW_HEIGHT + 2 * DROPDOWN_PADDING);

  const visibleItems = $derived.by(() => {
    const available = PILL_WIDTH - 2 * PADDING_X;
    const shown: PillItem[] = [];
    let used = 0;
    for (const [i, item] of model.items.entries()) {
      const width =
        ITEM_CHIP_BASE_WIDTH + item.label.length * ITEM_CHAR_WIDTH + (model.targetLocked ? 0 : ITEM_REMOVE_WIDTH);
      const overflow = i < model.items.length - 1 ? ITEM_GAP + ITEM_OVERFLOW_WIDTH : 0;
      if (used + width + overflow > available) break;
      shown.push(item);
      used += width + ITEM_GAP;
    }
    return { shown, hidden: model.items.length - shown.length };
  });

  function barScale(factor: number): number {
    return Math.max(MIN_BAR, Math.min(1, level * factor));
  }

  async function run(command: string, args?: Record<string, unknown>) {
    try {
      await invoke(command, args);
    } catch (e) {
      logError(`${command} failed: ${e}`);
    }
  }

  async function openDropdown() {
    if (model.targets.length === 0) return;
    try {
      const direction = await invoke<string>("set_recording_widget_dropdown", {
        open: true,
        rows: model.targets.length,
      });
      listUp = direction === "up";
      dropdownOpen = true;
    } catch (e) {
      logError(`set_recording_widget_dropdown failed: ${e}`);
    }
  }

  async function closeDropdown() {
    dropdownOpen = false;
    await run("set_recording_widget_dropdown", { open: false, rows: 0 });
  }

  async function selectTarget(id: string) {
    await run("select_recording_widget_target", { id });
    await closeDropdown();
  }

  $effect(() => {
    if (!interactive && dropdownOpen) closeDropdown();
  });

  onMount(async () => {
    attachConsole();
    unlisteners = await Promise.all([
      listen<WidgetPayload>("recording-widget-state", (event) => {
        widgetState = event.payload.state;
        elapsedMs = event.payload.elapsedMs;
        monochromatic = event.payload.monochromatic;
        if (widgetState === "recording") level = event.payload.level;
      }),
      listen<PillModel>("recording-widget-model", (event) => {
        model = event.payload;
      }),
      listen("recording-widget-dropdown-closed", () => {
        dropdownOpen = false;
      }),
    ]);
  });

  onDestroy(() => unlisteners.forEach((unlisten) => unlisten()));
</script>

<div class="root" class:up={listUp} style:--radius="{RADIUS}px">
  <div class="pill" class:mac={isMac} style:height="{pillHeight}px" style:padding="0 {PADDING_X}px" data-tauri-drag-region>
    {#if hasItems}
      <div class="row items" style:height="{ITEMS_ROW_HEIGHT}px" data-tauri-drag-region>
        {#each visibleItems.shown as item (item.index)}
          <span class="chip" style:gap="{ITEM_GAP}px">
            {#if item.kind === "image"}<ImageIcon size={12} />{:else}<Type size={12} />{/if}
            {item.label}
            {#if !model.targetLocked}
              <button
                type="button"
                class="chip-remove"
                tabindex="-1"
                aria-label="Remove {item.label}"
                onclick={() => run("remove_recording_widget_item", { index: item.index, contextVersion: model.contextVersion })}
              >
                <X size={12} />
              </button>
            {/if}
          </span>
        {/each}
        {#if visibleItems.hidden > 0}
          <span class="chip">+{visibleItems.hidden}</span>
        {/if}
      </div>
    {/if}
    <div class="row" style:height="{ROW_HEIGHT}px" data-tauri-drag-region>
      {#if widgetState === "recording" || widgetState === "paused"}
        <div class="bars" data-tauri-drag-region>
          {#each BAR_FACTORS as factor}
            <span class="bar" style:transform="scaleY({barScale(factor)})" data-tauri-drag-region></span>
          {/each}
        </div>
        <span class="time" data-tauri-drag-region>{clock}</span>
        <div class="actions">
          {#if widgetState === "recording"}
            <button type="button" tabindex="-1" aria-label="Pause" onclick={() => run("pause_speech_recording")}>
              <Pause size={16} />
            </button>
          {:else}
            <button type="button" tabindex="-1" aria-label="Resume" onclick={() => run("resume_speech_recording")}>
              <Play size={16} />
            </button>
          {/if}
          <button type="button" class="stop" tabindex="-1" aria-label="Stop" onclick={() => run("toggle_speech_recording")}>
            <Square size={16} />
          </button>
          <button type="button" tabindex="-1" aria-label="Cancel" onclick={() => run("cancel_speech_recording")}>
            <X size={16} />
          </button>
        </div>
      {:else if widgetState === "processing"}
        <div class="wave" style:--wave-period="{WAVE_PERIOD_MS}ms" data-tauri-drag-region>
          {#each WAVE_BARS as delayMs}
            <span class="wave-bar" style:animation-delay="{delayMs}ms" data-tauri-drag-region></span>
          {/each}
        </div>
      {:else}
        <span class="status" data-tauri-drag-region>
          <Check size={16} color={monochromatic ? "currentColor" : DONE_COLOR} />
          Copied
        </span>
      {/if}
    </div>
    <div class="row" style:height="{ROW_HEIGHT}px" data-tauri-drag-region>
      {#if interactive}
        <button
          type="button"
          class="target"
          tabindex="-1"
          aria-label="Target"
          onclick={() => (dropdownOpen ? closeDropdown() : openDropdown())}
        >
          <span class="label">{model.selectedLabel}</span>
          {#if model.dropdownUp}<ChevronUp size={14} />{:else}<ChevronDown size={14} />{/if}
        </button>
        <button type="button" tabindex="-1" aria-label="Paste" onclick={() => run("paste_into_recording_widget")}>
          <ClipboardPaste size={16} />
        </button>
        <button type="button" tabindex="-1" aria-label="Stop and chat" onclick={() => run("stop_chat_speech_recording")}>
          <MessageSquareShare size={16} />
        </button>
      {:else}
        <span class="label" data-tauri-drag-region>{model.selectedLabel}</span>
      {/if}
    </div>
  </div>
  {#if dropdownOpen}
    <div
      class="dropdown"
      class:mac={isMac}
      style:height="{dropdownHeight}px"
      style:padding="{DROPDOWN_PADDING}px 0"
    >
      {#each model.targets as target (target.id)}
        <button
          type="button"
          class="option"
          class:selected={target.id === model.selectedId}
          tabindex="-1"
          style:height="{DROPDOWN_ROW_HEIGHT}px"
          style:padding="0 {PADDING_X}px"
          onclick={() => selectTarget(target.id)}
        >
          {target.label}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    padding: 0;
    background: transparent;
    overflow: hidden;
    width: 100%;
    height: 100%;
    font-family: var(--font-sans);
    user-select: none;
  }

  .root {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .root.up {
    flex-direction: column-reverse;
    justify-content: flex-start;
  }

  .pill,
  .dropdown {
    box-sizing: border-box;
    flex: none;
    border-radius: var(--radius);
    background: #1c1c1e;
    color: #e5e5e7;
    font-size: 13px;
  }

  .pill.mac,
  .dropdown.mac {
    background: rgba(28, 28, 30, 0.92);
  }

  .pill {
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
  }

  .row.items {
    justify-content: flex-start;
    gap: 4px;
  }

  .chip {
    display: flex;
    align-items: center;
    height: 22px;
    padding: 0 6px;
    border-radius: 11px;
    background: rgba(255, 255, 255, 0.12);
    font-size: 11px;
    white-space: nowrap;
  }

  .chip-remove {
    width: 16px;
    height: 16px;
  }

  .target {
    flex: 1;
    min-width: 0;
    width: auto;
    height: 28px;
    gap: 4px;
    padding: 0 8px;
    border-radius: 14px;
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }

  .dropdown {
    display: flex;
    flex-direction: column;
  }

  .option {
    width: 100%;
    justify-content: flex-start;
    border-radius: 0;
    font: inherit;
  }

  .option.selected {
    font-weight: 700;
  }

  .bars {
    display: flex;
    align-items: center;
    gap: 3px;
    height: 24px;
  }

  .bar {
    width: 3px;
    height: 100%;
    border-radius: 2px;
    background: #e5e5e7;
    transition: transform 120ms ease-out;
  }

  .time {
    min-width: 34px;
    font-variant-numeric: tabular-nums;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: #e5e5e7;
    cursor: pointer;
  }

  button:hover {
    background: #3a3a3c;
  }

  button.stop {
    color: #d97373;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .wave {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 24px;
  }

  .wave-bar {
    width: 3px;
    height: 12px;
    border-radius: 2px;
    background: #929294;
    transform: scaleY(0.34);
    animation: wave var(--wave-period) ease-in-out infinite;
  }

  @keyframes wave {
    50% {
      transform: scaleY(1);
    }
  }
</style>
