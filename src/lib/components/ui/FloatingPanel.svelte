<script lang="ts">
  import type { Snippet } from "svelte";
  import { tick } from "svelte";

  let {
    visible,
    anchorEl,
    position = "auto",
    fitContent = false,
    onclose,
    children,
  }: {
    visible: boolean;
    anchorEl: HTMLElement | undefined;
    position?: "above" | "below" | "auto";
    fitContent?: boolean;
    onclose: () => void;
    children: Snippet;
  } = $props();

  let panelEl: HTMLDivElement | undefined = $state();
  let style = $state("");
  let armed = $state(false);
  let animating = $state(false);
  let expandDirection: "up" | "down" = $state("down");

  $effect(() => {
    if (visible && anchorEl) {
      armed = false;
      animating = true;
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
      expandDirection = "down";
    } else {
      top = anchorRect.top - panelHeight - 4;
      expandDirection = "up";
    }

    top = Math.max(4, Math.min(top, viewportHeight - panelHeight - 4));

    if (fitContent) {
      const left = Math.max(4, anchorRect.left);
      style = `top: ${top}px; left: ${left}px; right: auto; width: max-content;`;
    } else {
      style = `top: ${top}px`;
    }
  }

  function handlePointerDown(e: PointerEvent) {
    if (!visible || !armed || !panelEl) return;
    const target = e.target as HTMLElement;
    if (!panelEl.contains(target) && !anchorEl?.contains(target)) {
      onclose();
    }
  }

  function handleAnimationEnd() {
    animating = false;
  }
</script>

<svelte:window onpointerdown={handlePointerDown} />

{#if visible}
  <div
    class="floating-panel"
    class:expand-down={animating && expandDirection === "down"}
    class:expand-up={animating && expandDirection === "up"}
    class:flush={fitContent}
    bind:this={panelEl}
    {style}
    onanimationend={handleAnimationEnd}
  >
    {@render children()}
  </div>
{/if}

<style>
  @keyframes panel-expand-down {
    from {
      clip-path: inset(0 0 100% 0);
    }
    to {
      clip-path: inset(0 0 0 0);
    }
  }

  @keyframes panel-expand-up {
    from {
      clip-path: inset(100% 0 0 0);
    }
    to {
      clip-path: inset(0 0 0 0);
    }
  }

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

  .floating-panel.flush {
    padding: 4px 0;
    overflow: hidden;
  }

  .floating-panel.expand-down {
    animation: panel-expand-down 150ms ease-out;
  }

  .floating-panel.expand-up {
    animation: panel-expand-up 150ms ease-out;
  }
</style>
