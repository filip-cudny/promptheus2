<script lang="ts">
  import { X } from "lucide-svelte";
  import type { Snippet } from "svelte";

  type Variant = "default" | "small";

  let {
    label,
    readonly = false,
    variant = "default" as Variant,
    onclick,
    onremove,
    content,
  }: {
    label: string;
    readonly?: boolean;
    variant?: Variant;
    onclick: () => void;
    onremove: () => void;
    content: Snippet;
  } = $props();
</script>

<div class="chip-wrapper" class:small={variant === "small"}>
  <div class="chip">
    <button class="chip-btn" {onclick}>
      {@render content()}
      <span class="chip-badge">{label}</span>
    </button>
  </div>
  {#if !readonly}
    <button class="chip-delete" onclick={onremove}>
      <X size={11} strokeWidth={2.5} />
    </button>
  {/if}
</div>

<style>
  .chip-wrapper {
    position: relative;
    width: 120px;
    height: 120px;
    flex-shrink: 0;
  }

  .chip-wrapper.small {
    width: 64px;
    height: 64px;
  }

  .small .chip {
    border-radius: var(--radius-lg);
  }

  .small .chip-badge {
    font-size: 9px;
    padding: 1px 4px;
    bottom: 4px;
    left: 4px;
  }

  .small .chip-delete {
    width: 15px;
    height: 15px;
  }

  .chip {
    width: 100%;
    height: 100%;
    border-radius: var(--radius-xl);
    border: 1px solid var(--border-strong);
    background: rgba(255, 255, 255, 0.05);
    overflow: hidden;
  }

  .chip-btn {
    padding: var(--space-0);
    border: none;
    background: none;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    text-align: left;
    width: 100%;
    height: 100%;
  }

  .chip-badge {
    position: absolute;
    bottom: 6px;
    left: 6px;
    font-size: 9px;
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.8px;
    color: var(--text-primary);
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 1px 4px;
  }

  .chip-delete {
    position: absolute;
    top: -3px;
    right: -3px;
    width: 19px;
    height: 19px;
    border-radius: 50%;
    border: 1px solid var(--border-hard);
    background: var(--surface-elevated);
    color: var(--text-primary);
    cursor: pointer;
    display: grid;
    place-items: center;
    padding: var(--space-0);
    box-shadow: var(--shadow-sm);
    opacity: 0;
    transition: opacity var(--motion-default) var(--ease-default);
  }

  .chip-wrapper:hover .chip-delete {
    opacity: 1;
  }

  .chip-delete:hover {
    background: var(--surface-elevated);
  }
</style>
