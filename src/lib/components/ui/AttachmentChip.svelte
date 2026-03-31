<script lang="ts">
  import { X } from "lucide-svelte";
  import type { Snippet } from "svelte";

  let {
    label,
    readonly = false,
    onclick,
    onremove,
    content,
  }: {
    label: string;
    readonly?: boolean;
    onclick: () => void;
    onremove: () => void;
    content: Snippet;
  } = $props();
</script>

<div class="chip-wrapper">
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

  .chip {
    width: 100%;
    height: 100%;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: rgba(255, 255, 255, 0.05);
    overflow: hidden;
  }

  .chip-btn {
    padding: 0;
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
    font-size: 8px;
    font-weight: 700;
    letter-spacing: 0.8px;
    color: rgba(255, 255, 255, 0.85);
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    border: 1px solid rgba(255, 255, 255, 0.25);
    border-radius: 3px;
    padding: 1px 4px;
  }

  .chip-delete {
    position: absolute;
    top: -3px;
    right: -3px;
    width: 19px;
    height: 19px;
    border-radius: 50%;
    border: 1px solid #555;
    background: #333;
    color: #fff;
    cursor: pointer;
    display: grid;
    place-items: center;
    padding: 0;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .chip-wrapper:hover .chip-delete {
    opacity: 1;
  }

  .chip-delete:hover {
    background: #444;
  }
</style>
