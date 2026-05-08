<script lang="ts">
  import { Check, AlertCircle } from "lucide-svelte";
  import type { SaveTracker } from "$lib/stores/saveTracker.svelte";

  let {
    tracker,
    compact = false,
  }: {
    tracker: SaveTracker;
    compact?: boolean;
  } = $props();

  async function copyError() {
    if (!tracker.error) return;
    try {
      await navigator.clipboard.writeText(tracker.error);
    } catch {
      // ignore
    }
  }
</script>

<span class="save-status">
  <span
    class="status-dot"
    class:dirty={tracker.state === "dirty"}
    class:saving={tracker.state === "saving"}
    class:saved={tracker.state === "saved"}
    class:err={tracker.state === "error"}
    title={tracker.tooltip}
    aria-label={tracker.tooltip}
  ></span>
  {#if !compact && tracker.state === "saved"}
    <span class="saved-stamp" aria-hidden="true">
      <Check size={10} /> saved
    </span>
  {/if}
  {#if tracker.state === "error"}
    <button
      type="button"
      class="err-stamp"
      title={tracker.error ?? ""}
      onclick={copyError}
      aria-label="Copy error message"
    >
      <AlertCircle size={10} /> save failed
    </button>
  {/if}
</span>

<style>
  .save-status {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-faint);
    transition:
      background var(--motion-fast) var(--ease-default),
      transform var(--motion-fast) var(--ease-default);
    cursor: help;
    flex-shrink: 0;
  }

  .status-dot.dirty {
    background: var(--accent);
  }

  .status-dot.saving {
    background: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }

  .status-dot.saved {
    background: var(--success);
  }

  .status-dot.err {
    background: var(--danger);
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 0.55;
    }
    50% {
      opacity: 1;
    }
  }

  .saved-stamp,
  .err-stamp {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: var(--font-size-xs);
    color: var(--text-muted);
    animation: fade-in var(--motion-default) var(--ease-default);
  }

  .saved-stamp {
    color: var(--success);
  }

  .err-stamp {
    color: var(--danger);
    background: transparent;
    border: none;
    padding: 0;
    font: inherit;
    cursor: pointer;
  }

  .err-stamp:hover {
    text-decoration: underline;
  }

  @keyframes fade-in {
    from {
      opacity: 0;
      transform: translateY(-2px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
