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

<button class="tool-chip" onclick={ondismiss} title="Remove {label}">
  {#if icon}
    {@const Icon = icon}
    <Icon size={12} />
  {/if}
  <span class="tool-chip-label">{label}</span>
  <X size={10} />
</button>

<style>
  .tool-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 6px 0 8px;
    background: #5b8dd9;
    color: white;
    border: none;
    border-radius: 10px;
    font: inherit;
    font-size: 11px;
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .tool-chip:hover {
    background: #4a7cc8;
  }

  .tool-chip-label {
    line-height: 1;
  }
</style>
