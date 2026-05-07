<script lang="ts">
  import type { Snippet } from "svelte";
  import { tick } from "svelte";

  let {
    visible,
    anchorEl,
    position = "auto",
    fitContent = false,
    flush,
    onclose,
    children,
  }: {
    visible: boolean;
    anchorEl: HTMLElement | undefined;
    position?: "above" | "below" | "auto";
    fitContent?: boolean;
    flush?: boolean;
    onclose: () => void;
    children: Snippet;
  } = $props();

  let isFlush = $derived(flush ?? fitContent);

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
    const panelWidth = panelEl.offsetWidth;
    const viewportHeight = window.innerHeight;
    const viewportWidth = window.innerWidth;

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
      const maxLeft = Math.max(4, viewportWidth - panelWidth - 4);
      const left = Math.max(4, Math.min(anchorRect.left, maxLeft));
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
    class:flush={isFlush}
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
    z-index: var(--z-overlay);
    background: var(--surface-floating-popover);
    border: 1px solid var(--surface-floating-popover-border);
    border-radius: var(--radius-lg);
    padding: var(--space-4) var(--space-6);
    box-shadow: var(--shadow-md);
    box-sizing: border-box;
  }

  .floating-panel.flush {
    padding: var(--space-2) var(--space-0);
    overflow: hidden;
  }

  .floating-panel.expand-down {
    animation: panel-expand-down 150ms ease-out;
  }

  .floating-panel.expand-up {
    animation: panel-expand-up 150ms ease-out;
  }
</style>
