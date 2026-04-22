<script lang="ts">
  import { onDestroy } from "svelte";
  import { slide } from "svelte/transition";
  import { ChevronRight, ChevronDown, Brain } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";
  import MarkdownRenderer from "./MarkdownRenderer.svelte";

  let {
    thinkingContent,
    isThinkingActive,
    isStreaming = false,
    thinkingDuration = null,
  }: {
    thinkingContent: string;
    isThinkingActive: boolean;
    isStreaming: boolean;
    thinkingDuration?: number | null;
  } = $props();

  let expanded = $state(false);
  let startTime: number | null = $state(null);
  let elapsed = $state(0);
  let finalElapsed: number | null = $state(null);
  let intervalId: ReturnType<typeof setInterval> | null = null;
  let liveThinkingEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (liveThinkingEl && thinkingContent) {
      liveThinkingEl.scrollTop = liveThinkingEl.scrollHeight;
    }
  });

  $effect(() => {
    if (isThinkingActive) {
      if (startTime === null) {
        startTime = Date.now();
        finalElapsed = null;
      }
      intervalId = setInterval(() => {
        elapsed = Math.floor((Date.now() - startTime!) / 1000);
      }, 1000);
      return () => {
        if (intervalId) clearInterval(intervalId);
      };
    } else if (startTime !== null && finalElapsed === null) {
      finalElapsed = elapsed;
      startTime = null;
      if (intervalId) {
        clearInterval(intervalId);
        intervalId = null;
      }
    }
  });

  onDestroy(() => {
    if (intervalId) clearInterval(intervalId);
  });

  function formatElapsed(seconds: number): string {
    if (seconds < 60) return `${seconds}s`;
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return s > 0 ? `${m}m ${s}s` : `${m}m`;
  }

  let showTimer = $derived(elapsed >= 3);
  let hasContent = $derived(thinkingContent.length > 0);
</script>

{#if isThinkingActive}
  <div class="thinking-block" role="status" aria-live="polite">
    <div class="thinking-active">
      <Brain size={ICON_SIZE.sm} />
      <span class="thinking-label">Thinking</span>
      {#if showTimer}
        <span class="thinking-timer">{formatElapsed(elapsed)}</span>
      {/if}
    </div>
    {#if hasContent}
      <div class="thinking-content thinking-content--live" bind:this={liveThinkingEl}>
        <MarkdownRenderer content={thinkingContent} isStreaming={true} />
      </div>
    {/if}
  </div>
{:else if hasContent}
  <div class="thinking-block">
    <button
      class="thinking-toggle"
      onclick={() => (expanded = !expanded)}
      aria-expanded={expanded}
    >
      {#if expanded}
        <ChevronDown size={ICON_SIZE.sm} />
      {:else}
        <ChevronRight size={ICON_SIZE.sm} />
      {/if}
      <Brain size={ICON_SIZE.sm} />
      <span class="thinking-summary">
        Thought{#if finalElapsed != null}&nbsp;for {formatElapsed(finalElapsed)}{:else if thinkingDuration != null}&nbsp;for {formatElapsed(thinkingDuration)}{/if}
      </span>
    </button>
    {#if expanded}
      <div class="thinking-content" transition:slide={{ duration: 150 }}>
        <MarkdownRenderer content={thinkingContent} isStreaming={false} />
      </div>
    {/if}
  </div>
{/if}

<style>
  .thinking-block {
    margin: 4px 0;
    border-left: 3px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.03);
    overflow: hidden;
    color: rgba(255, 255, 255, 0.7);
  }

  .thinking-active {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
  }

  .thinking-label {
    font-size: 13px;
    font-weight: 600;
    background: linear-gradient(
      90deg,
      rgba(155, 109, 204, 0.6) 0%,
      rgba(200, 170, 240, 0.9) 50%,
      rgba(155, 109, 204, 0.6) 100%
    );
    background-size: 200% auto;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    animation: shimmer 2s linear infinite;
  }

  @keyframes shimmer {
    0% { background-position: -200% center; }
    100% { background-position: 200% center; }
  }

  @media (prefers-reduced-motion: reduce) {
    .thinking-label {
      animation: none;
      background: none;
      -webkit-text-fill-color: #c9a5f0;
    }
  }

  .thinking-timer {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.35);
  }

  .thinking-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    border: none;
    background: none;
    color: rgba(255, 255, 255, 0.7);
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }

  .thinking-toggle:hover {
    background: rgba(255, 255, 255, 0.04);
    color: rgba(255, 255, 255, 0.9);
  }

  .thinking-summary {
    font-weight: 500;
  }

  .thinking-content {
    max-height: 400px;
    overflow-y: auto;
    padding: 4px 10px 10px 10px;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.6);
  }

  .thinking-content--live {
    max-height: 300px;
    overflow-anchor: auto;
  }
</style>
