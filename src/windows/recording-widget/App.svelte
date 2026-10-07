<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { attachConsole, error as logError } from "@tauri-apps/plugin-log";
  import { Check, Pause, Play, Square, X } from "lucide-svelte";

  type WidgetState = "recording" | "paused" | "processing" | "done";

  interface WidgetPayload {
    state: WidgetState;
    level: number;
    elapsedMs: number;
    monochromatic: boolean;
  }

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
  let unlisten: UnlistenFn | undefined;

  const clock = $derived.by(() => {
    const totalSeconds = Math.floor(elapsedMs / 1000);
    const seconds = String(totalSeconds % 60).padStart(2, "0");
    return `${Math.floor(totalSeconds / 60)}:${seconds}`;
  });

  function barHeight(factor: number): number {
    return Math.max(MIN_BAR, Math.min(1, level * factor)) * 100;
  }

  async function run(command: string) {
    try {
      await invoke(command);
    } catch (e) {
      logError(`${command} failed: ${e}`);
    }
  }

  onMount(async () => {
    attachConsole();
    unlisten = await listen<WidgetPayload>("recording-widget-state", (event) => {
      widgetState = event.payload.state;
      elapsedMs = event.payload.elapsedMs;
      monochromatic = event.payload.monochromatic;
      if (widgetState === "recording") level = event.payload.level;
    });
  });

  onDestroy(() => unlisten?.());
</script>

<div class="pill" class:mac={isMac} data-tauri-drag-region>
  {#if widgetState === "recording" || widgetState === "paused"}
    <div class="bars" data-tauri-drag-region>
      {#each BAR_FACTORS as factor}
        <span class="bar" style:height="{barHeight(factor)}%" data-tauri-drag-region></span>
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

  .pill {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 0 12px;
    border-radius: 9999px;
    background: #1c1c1e;
    color: #e5e5e7;
    font-size: 13px;
  }

  .pill.mac {
    background: rgba(28, 28, 30, 0.92);
  }

  .bars {
    display: flex;
    align-items: center;
    gap: 3px;
    height: 24px;
  }

  .bar {
    width: 3px;
    border-radius: 2px;
    background: #e5e5e7;
    transition: height 90ms ease-out;
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
