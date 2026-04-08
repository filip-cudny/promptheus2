<script lang="ts">
  import type { Snippet } from "svelte";
  import { tick } from "svelte";

  let {
    visible,
    anchorEl,
    position = "auto",
    onclose,
    children,
  }: {
    visible: boolean;
    anchorEl: HTMLElement | undefined;
    position?: "above" | "below" | "auto";
    onclose: () => void;
    children: Snippet;
  } = $props();

  let panelEl: HTMLDivElement | undefined = $state();
  let style = $state("");
  let armed = $state(false);

  $effect(() => {
    if (visible && anchorEl) {
      armed = false;
      tick().then(() => {
        computePosition();
        armed = true;
      });
    } else {
      armed = false;
    }
  });

  function computePosition() {
    if (!anchorEl || !panelEl) return;

    const anchorRect = anchorEl.getBoundingClientRect();
    const panelHeight = panelEl.offsetHeight;
    const viewportHeight = window.innerHeight;

    const spaceBelow = viewportHeight - anchorRect.bottom;

    let top: number;
    if (position === "below" || (position === "auto" && spaceBelow >= panelHeight + 4)) {
      top = anchorRect.bottom + 4;
    } else {
      top = anchorRect.top - panelHeight - 4;
    }

    top = Math.max(4, Math.min(top, viewportHeight - panelHeight - 4));
    style = `top: ${top}px`;
  }

  function handlePointerDown(e: PointerEvent) {
    if (!visible || !armed || !panelEl) return;
    const target = e.target as HTMLElement;
    if (!panelEl.contains(target) && !anchorEl?.contains(target)) {
      onclose();
    }
  }

</script>

<svelte:window onpointerdown={handlePointerDown} />

{#if visible}
  <div class="floating-panel" bind:this={panelEl} {style}>
    {@render children()}
  </div>
{/if}

<style>
  .floating-panel {
    position: fixed;
    left: 4px;
    right: 4px;
    z-index: 1000;
    background: #252525;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    padding: 8px 12px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    box-sizing: border-box;
  }
</style>
