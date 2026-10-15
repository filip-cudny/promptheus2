<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { ContextItem } from "$lib/types/context";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { attachConsole, error as logError } from "@tauri-apps/plugin-log";
  import {
    Check,
    ChevronsUpDown,
    ClipboardPaste,
    Image as ImageIcon,
    MessageSquareShare,
    Pause,
    Play,
    Type,
    X,
  } from "lucide-svelte";
  import claudeIcon from "$lib/icons/providers/claude.svg?raw";
  import chatgptIcon from "$lib/icons/providers/chatgpt.svg?raw";

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
    preview: string;
  }

  type PreviewContent = { kind: "text"; text: string } | { kind: "image"; src: string };

  interface PillModel {
    targets: PillTarget[];
    selectedId: string;
    selectedLabel: string;
    targetLocked: boolean;
    items: PillItem[];
    itemRows: number;
    contextVersion: number;
  }

  const PILL_WIDTH = 264;
  const PILL_HEIGHT = 72;
  const MAX_ITEM_ROWS = 3;
  const ITEM_ROW_HEIGHT = 22;
  const ITEM_ROW_GAP = 4;
  const ITEMS_ROW_PADDING = 10;
  const PREVIEW_MAX_HEIGHT = 160;
  const PREVIEW_GAP = 6;
  const PREVIEW_DELAY_MS = 300;
  const ROW_HEIGHT = PILL_HEIGHT / 2;
  const RADIUS = 16;
  const PADDING_X = 14;
  const CHIP_GAP = 4;
  const DROPDOWN_ROW_HEIGHT = 28;
  const DROPDOWN_PADDING = 4;
  const TARGET_ICONS: Record<string, string> = { "webview:claude": claudeIcon, "webview:chatgpt": chatgptIcon };
  const TARGET_ICON_SIZE = 14;
  const TARGET_ICON_GAP = 8;
  const DEFAULT_TARGET_LABEL = "Dictation";
  const BAR_WIDTH = 2;
  const BAR_GAP = 3;
  const TIME_WIDTH = 34;
  const ROW_GAP = 10;
  const BUTTON_SIZE = 28;
  const ACTION_GAP = 4;
  const ROW1_BUTTONS = 3;
  const METER_WIDTH =
    PILL_WIDTH - 2 * PADDING_X - TIME_WIDTH - 2 * ROW_GAP - ROW1_BUTTONS * BUTTON_SIZE - (ROW1_BUTTONS - 1) * ACTION_GAP;
  const BAR_COUNT = Math.floor((METER_WIDTH + BAR_GAP) / (BAR_WIDTH + BAR_GAP));
  const BAR_PATTERN = [0.5, 0.8, 0.65, 1, 0.7, 0.9, 0.55, 0.95, 0.75, 0.85, 0.6, 0.45, 0.8, 0.95, 0.6, 0.7, 1, 0.55];
  const BAR_FACTORS = Array.from({ length: BAR_COUNT }, (_, i) => BAR_PATTERN[i % BAR_PATTERN.length]);
  const MIN_BAR = 0.12;
  const DONE_COLOR = "#62A878";
  const WAVE_BAR_WIDTH = 2;
  const WAVE_BAR_COUNT = Math.floor((PILL_WIDTH - 2 * PADDING_X + BAR_GAP) / (WAVE_BAR_WIDTH + BAR_GAP));
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
    itemRows: 0,
    contextVersion: 0,
  });
  let dropdownOpen = $state(false);
  let overlayUp = $state(false);
  let previewContent = $state<PreviewContent | null>(null);
  let hoveredIndex: number | null = null;
  let hoveredButton: HTMLButtonElement | null = null;
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;
  let previewToken = 0;
  let previewRequested = false;
  let previewOpenInBackend = false;
  let previewQueue: Promise<void> = Promise.resolve();
  let contextItems: ContextItem[] | null = null;
  let unlisteners: UnlistenFn[] = [];

  const clock = $derived.by(() => {
    const totalSeconds = Math.floor(elapsedMs / 1000);
    const seconds = String(totalSeconds % 60).padStart(2, "0");
    return `${Math.floor(totalSeconds / 60)}:${seconds}`;
  });

  const interactive = $derived(!model.targetLocked && widgetState !== "done");
  const hasItems = $derived(model.items.length > 0);
  const pillHeight = $derived(PILL_HEIGHT + itemsHeight(model.itemRows));
  const dropdownHeight = $derived(model.targets.length * DROPDOWN_ROW_HEIGHT + 2 * DROPDOWN_PADDING);

  function itemsHeight(rows: number): number {
    if (rows <= 0) return 0;
    const clamped = Math.min(rows, MAX_ITEM_ROWS);
    return clamped * ITEM_ROW_HEIGHT + (clamped - 1) * ITEM_ROW_GAP + ITEMS_ROW_PADDING;
  }

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

  function enqueuePreviewCall(task: () => Promise<void>) {
    previewQueue = previewQueue.then(task);
  }

  function resetPreviewState() {
    clearTimeout(hoverTimer);
    hoveredIndex = null;
    previewToken++;
    previewContent = null;
    previewRequested = false;
    previewOpenInBackend = false;
  }

  function hidePreview() {
    clearTimeout(hoverTimer);
    hoveredIndex = null;
    previewToken++;
    previewContent = null;
    if (!previewRequested) return;
    previewRequested = false;
    enqueuePreviewCall(async () => {
      await run("set_recording_widget_preview", { open: false });
      previewOpenInBackend = false;
    });
  }

  async function loadPreviewContent(item: PillItem): Promise<PreviewContent | null> {
    if (item.kind === "text") return { kind: "text", text: item.preview };
    try {
      contextItems ??= await invoke<ContextItem[]>("get_context_items");
    } catch (e) {
      logError(`get_context_items failed: ${e}`);
      return null;
    }
    const source = contextItems[item.index];
    if (source?.item_type !== "image") return null;
    return { kind: "image", src: `data:${source.media_type};base64,${source.data}` };
  }

  async function showPreview(index: number) {
    const item = model.items.find((candidate) => candidate.index === index);
    if (!item || dropdownOpen) return;
    const token = ++previewToken;
    const content = await loadPreviewContent(item);
    if (!content || token !== previewToken || dropdownOpen) return;
    previewRequested = true;
    enqueuePreviewCall(async () => {
      if (token !== previewToken) return;
      if (!previewOpenInBackend) {
        try {
          const direction = await invoke<string>("set_recording_widget_preview", { open: true });
          previewOpenInBackend = true;
          overlayUp = direction === "up";
        } catch (e) {
          logError(`set_recording_widget_preview failed: ${e}`);
          previewRequested = false;
          return;
        }
      }
      if (token === previewToken) previewContent = content;
    });
  }

  function onPointer(point: { x: number; y: number } | null) {
    const element = point ? document.elementFromPoint(point.x, point.y) : null;
    const button = element?.closest("button") ?? null;
    if (button !== hoveredButton) {
      hoveredButton?.classList.remove("hover");
      button?.classList.add("hover");
      hoveredButton = button;
    }
    const chip = element?.closest<HTMLElement>("[data-item-index]");
    if (!chip) {
      hidePreview();
      return;
    }
    const index = Number(chip.dataset.itemIndex);
    if (index === hoveredIndex) return;
    clearTimeout(hoverTimer);
    hoveredIndex = index;
    hoverTimer = setTimeout(() => showPreview(index), PREVIEW_DELAY_MS);
  }

  async function openDropdown() {
    if (model.targets.length === 0) return;
    hidePreview();
    try {
      const direction = await invoke<string>("set_recording_widget_dropdown", {
        open: true,
        rows: model.targets.length,
      });
      overlayUp = direction === "up";
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
        contextItems = null;
      }),
      listen<{ x: number; y: number } | null>("recording-widget-pointer", (event) => onPointer(event.payload)),
      listen("recording-widget-dropdown-closed", () => {
        dropdownOpen = false;
        resetPreviewState();
      }),
    ]);
  });

  onDestroy(() => unlisteners.forEach((unlisten) => unlisten()));
</script>

<div class="root" class:up={overlayUp} style:--radius="{RADIUS}px">
  <div class="pill" class:mac={isMac} style:height="{pillHeight}px" style:padding="0 {PADDING_X}px" data-tauri-drag-region>
    {#if hasItems}
      <div class="items" style:height="{itemsHeight(model.itemRows)}px" style:gap="{ITEM_ROW_GAP}px" data-tauri-drag-region>
        {#each model.items as item (item.index)}
          <span class="chip" style:gap="{CHIP_GAP}px" data-item-index={item.index}>
            {#if item.kind === "image"}<ImageIcon size={12} />{:else}<Type size={12} />{/if}
            <span class="chip-label">{item.label}</span>
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
      </div>
    {/if}
    <div class="row" style:height="{ROW_HEIGHT}px" data-tauri-drag-region>
      {#if widgetState === "recording" || widgetState === "paused"}
        <div class="bars" class:paused={widgetState === "paused"} style:gap="{BAR_GAP}px" data-tauri-drag-region>
          {#each BAR_FACTORS as factor}
            <span class="bar" style:width="{BAR_WIDTH}px" style:transform="scaleY({barScale(factor)})" data-tauri-drag-region></span>
          {/each}
        </div>
        <span class="time" style:min-width="{TIME_WIDTH}px" data-tauri-drag-region>{clock}</span>
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
          <button type="button" class="cancel" tabindex="-1" aria-label="Cancel" onclick={() => run("cancel_speech_recording")}>
            <X size={16} />
          </button>
          <button type="button" tabindex="-1" aria-label="Finish" onclick={() => run("toggle_speech_recording")}>
            <Check size={16} color={monochromatic ? "currentColor" : DONE_COLOR} />
          </button>
        </div>
      {:else if widgetState === "processing"}
        <div class="wave" style:--wave-period="{WAVE_PERIOD_MS}ms" style:gap="{BAR_GAP}px" data-tauri-drag-region>
          {#each WAVE_BARS as delayMs}
            <span class="wave-bar" style:width="{WAVE_BAR_WIDTH}px" style:animation-delay="{delayMs}ms" data-tauri-drag-region></span>
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
          <ChevronsUpDown size={14} />
        </button>
        <div class="actions">
          <button type="button" tabindex="-1" aria-label="Paste" onclick={() => run("paste_into_recording_widget")}>
            <ClipboardPaste size={16} />
          </button>
          <button type="button" tabindex="-1" aria-label="Stop and chat" onclick={() => run("stop_chat_speech_recording")}>
            <MessageSquareShare size={16} />
          </button>
        </div>
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
      style:--row-radius="{RADIUS - DROPDOWN_PADDING}px"
    >
      {#each model.targets as target (target.id)}
        <button
          type="button"
          class="option"
          class:selected={target.id === model.selectedId}
          tabindex="-1"
          style:height="{DROPDOWN_ROW_HEIGHT}px"
          style:padding="0 {PADDING_X}px"
          style:gap="{TARGET_ICON_GAP}px"
          onclick={() => selectTarget(target.id)}
        >
          {#if TARGET_ICONS[target.id]}
            <span class="option-icon" style:--icon-size="{TARGET_ICON_SIZE}px" aria-hidden="true">{@html TARGET_ICONS[target.id]}</span>
          {/if}
          {target.label}
        </button>
      {/each}
    </div>
  {/if}
  {#if previewContent}
    <div
      class="preview"
      class:image={previewContent.kind === "image"}
      style:max-height="{PREVIEW_MAX_HEIGHT}px"
      style:margin-top={overlayUp ? undefined : `${PREVIEW_GAP}px`}
      style:margin-bottom={overlayUp ? `${PREVIEW_GAP}px` : undefined}
    >
      {#if previewContent.kind === "image"}
        <img src={previewContent.src} alt="" />
      {:else}
        {previewContent.text}
      {/if}
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
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.12);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
  }

  .items {
    display: grid;
    grid-template-columns: 1fr 1fr;
    align-content: start;
    box-sizing: border-box;
    padding: 5px 0;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .items::-webkit-scrollbar {
    display: none;
  }

  .preview {
    box-sizing: border-box;
    flex: none;
    overflow: hidden;
    padding: 8px;
    border: 1px solid var(--surface-floating-popover-border);
    border-radius: var(--surface-floating-popover-radius);
    background: var(--surface-floating-popover);
    color: #e5e5e7;
    font-size: 12px;
    white-space: pre-wrap;
  }

  .preview.image {
    height: 160px;
    padding: 0;
  }

  .preview img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .chip {
    display: flex;
    align-items: center;
    height: 22px;
    min-width: 0;
    padding: 0 6px;
    border-radius: 11px;
    background: rgba(255, 255, 255, 0.12);
    font-size: 11px;
    white-space: nowrap;
  }

  .chip-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip-remove {
    flex: none;
    width: 16px;
    height: 16px;
  }

  .target {
    flex: 0 1 auto;
    min-width: 0;
    width: auto;
    height: 28px;
    gap: 4px;
    margin-left: -8px;
    padding: 0 8px;
    border-radius: 14px;
    justify-content: flex-start;
    color: #929294;
  }

  .label {
    color: #929294;
    flex: 0 1 auto;
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

  .option:first-child {
    border-top-left-radius: var(--row-radius);
    border-top-right-radius: var(--row-radius);
  }

  .option:last-child {
    border-bottom-left-radius: var(--row-radius);
    border-bottom-right-radius: var(--row-radius);
  }

  .option-icon {
    display: flex;
    flex: none;
    width: var(--icon-size);
    height: var(--icon-size);
  }

  .option-icon :global(svg) {
    width: 100%;
    height: 100%;
  }

  .option.selected {
    font-weight: 700;
  }

  .bars {
    display: flex;
    align-items: center;
    height: 28px;
  }

  .bar {
    height: 100%;
    border-radius: 2px;
    background: #e5e5e7;
    transition: transform 120ms ease-out;
  }

  .bars.paused .bar {
    background: #929294;
  }

  .time {
    font-variant-numeric: tabular-nums;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
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

  button:hover,
  button:global(.hover) {
    background: #3a3a3c;
  }

  button.cancel:hover,
  button.cancel:global(.hover) {
    color: #d97373;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    animation: status-enter 180ms cubic-bezier(0.33, 1, 0.68, 1);
  }

  @keyframes status-enter {
    from {
      opacity: 0;
      transform: scale(0.85);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .status {
      animation: none;
    }
  }

  .wave {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 24px;
  }

  .wave-bar {
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
