<script lang="ts">
  import { MousePointer2 } from "lucide-svelte";
  import type { PillAnchor } from "$lib/types";

  interface Props {
    value: PillAnchor;
    disabled: boolean;
    onchange: (anchor: PillAnchor) => void;
  }

  let { value, disabled, onchange }: Props = $props();

  const ABOVE: { anchor: PillAnchor; label: string }[] = [
    { anchor: "above_left", label: "Above, extending left" },
    { anchor: "above_center", label: "Above, centred" },
    { anchor: "above_right", label: "Above, extending right" },
  ];
  const BELOW: { anchor: PillAnchor; label: string }[] = [
    { anchor: "below_left", label: "Below, extending left" },
    { anchor: "below_center", label: "Below, centred" },
    { anchor: "below_right", label: "Below, extending right" },
  ];
</script>

{#snippet tiles(options: { anchor: PillAnchor; label: string }[])}
  {#each options as opt (opt.anchor)}
    <label class="tile {opt.anchor}" class:active={value === opt.anchor} title={opt.label}>
      <input
        type="radio"
        name="recording-widget-anchor"
        class="sr-only"
        aria-label={opt.label}
        checked={value === opt.anchor}
        onchange={() => onchange(opt.anchor)}
      />
      <span class="pill"></span>
    </label>
  {/each}
{/snippet}

<fieldset class="picker" {disabled}>
  <legend>Widget position relative to the cursor</legend>
  <div class="grid">
    {@render tiles(ABOVE)}
    <span class="cursor"><MousePointer2 size={14} /></span>
    {@render tiles(BELOW)}
  </div>
</fieldset>

<style>
  .picker {
    border: none;
    margin: 0;
    padding: 0;
    min-width: 0;
  }

  legend {
    padding: 0;
    margin-bottom: var(--space-3);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(3, 72px);
    gap: var(--space-3);
  }

  .cursor {
    grid-column: 2;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    border: 0;
  }

  .tile {
    position: relative;
    height: 36px;
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
    cursor: pointer;
    transition: border-color var(--motion-fast) var(--ease-default);
  }

  .tile:hover {
    border-color: var(--border-strong);
  }

  .tile:has(input:focus-visible) {
    outline: 2px solid var(--accent-ring);
    outline-offset: 1px;
  }

  .tile.active {
    border-color: var(--accent-border);
    background: var(--accent-bg-soft);
  }

  .pill {
    position: absolute;
    width: 32px;
    height: 10px;
    border-radius: var(--radius-full);
    background: var(--text-muted);
  }

  .tile.active .pill {
    background: var(--accent);
  }

  .tile[class*="above_"] .pill {
    bottom: var(--space-3);
  }

  .tile[class*="below_"] .pill {
    top: var(--space-3);
  }

  .tile[class*="_right"] .pill {
    left: 50%;
  }

  .tile[class*="_center"] .pill {
    left: 50%;
    transform: translateX(-50%);
  }

  .tile[class*="_left"] .pill {
    right: 50%;
  }

  .picker:disabled {
    opacity: var(--opacity-disabled);
  }

  .picker:disabled .tile {
    cursor: default;
  }
</style>
