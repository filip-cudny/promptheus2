<script lang="ts">
  import { X } from "lucide-svelte";
  import type { ComponentType, SvelteComponent } from "svelte";
  import type { IconProps } from "lucide-svelte";

  type LucideIcon = ComponentType<SvelteComponent<IconProps>>;

  let {
    label,
    icon,
    ondismiss,
  }: {
    label: string;
    icon?: LucideIcon;
    ondismiss: () => void;
  } = $props();
</script>

<button class="tool-chip" title={label} onclick={ondismiss}>
  {#if icon}
    {@const Icon = icon}
    <Icon size={14} />
  {:else}
    <span class="tool-chip-label">{label}</span>
  {/if}
  <span class="tool-chip-dismiss">
    <X size={10} />
  </span>
</button>

<style>
  .tool-chip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0;
    width: 24px;
    height: 24px;
    padding: 0;
    background: rgba(91, 141, 217, 0.15);
    border: 1px solid rgba(91, 141, 217, 0.35);
    color: #5b8dd9;
    border-radius: 6px;
    font: inherit;
    font-size: 11px;
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
    overflow: hidden;
    transition: background 0.15s ease, width 0.15s ease;
  }

  .tool-chip:hover {
    background: rgba(91, 141, 217, 0.25);
    width: 36px;
  }

  .tool-chip-label {
    line-height: 1;
  }

  .tool-chip-dismiss {
    display: inline-flex;
    align-items: center;
    width: 0;
    opacity: 0;
    overflow: hidden;
    transition: width 0.15s ease, opacity 0.15s ease, margin 0.15s ease;
    margin-left: 0;
    color: #5b8dd9;
    pointer-events: none;
  }

  .tool-chip:hover .tool-chip-dismiss {
    width: 10px;
    opacity: 1;
    margin-left: 2px;
  }
</style>
