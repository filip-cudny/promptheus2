<script lang="ts">
  import type { Snippet } from "svelte";
  import { tick } from "svelte";

  let {
    visible,
    anchorEl,
    position = "auto",
    align = "start",
    fitContent = false,
    flush,
    onclose,
    children,
  }: {
    visible: boolean;
    anchorEl: HTMLElement | undefined;
    position?: "above" | "below" | "auto";
    align?: "start" | "end";
    fitContent?: boolean;
    flush?: boolean;
    onclose: () => void;
    children: Snippet;
  } = $props();

  const GAP = 4;
  const VIEWPORT_PADDING = 4;
  const MIN_CLAMPED_HEIGHT = 80;

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
      style = "";
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

    const spaceAbove = anchorRect.top - GAP - VIEWPORT_PADDING;
    const spaceBelow = viewportHeight - anchorRect.bottom - GAP - VIEWPORT_PADDING;

    let up: boolean;
    if (position === "below") up = false;
    else if (position === "above") up = spaceAbove >= panelHeight || spaceAbove >= spaceBelow;
    else up = spaceBelow < panelHeight && spaceAbove > spaceBelow;
    expandDirection = up ? "up" : "down";

    const available = Math.max(MIN_CLAMPED_HEIGHT, up ? spaceAbove : spaceBelow);
    const height = Math.min(panelHeight, available);
    const clamp = panelHeight > available ? ` max-height: ${available}px; overflow-y: auto;` : "";

    const top = up ? anchorRect.top - GAP - height : anchorRect.bottom + GAP;
    const clampedTop = Math.max(VIEWPORT_PADDING, Math.min(top, viewportHeight - height - VIEWPORT_PADDING));

    if (fitContent) {
      const maxLeft = Math.max(VIEWPORT_PADDING, viewportWidth - panelWidth - VIEWPORT_PADDING);
      const anchorLeft = align === "end" ? anchorRect.right - panelWidth : anchorRect.left;
      const left = Math.max(VIEWPORT_PADDING, Math.min(anchorLeft, maxLeft));
      style = `top: ${clampedTop}px; left: ${left}px;${clamp}`;
    } else {
      style = `top: ${clampedTop}px;${clamp}`;
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
    class:fit={fitContent}
    class:armed
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

  .floating-panel.fit {
    right: auto;
    width: max-content;
  }

  .floating-panel:not(.armed) {
    visibility: hidden;
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
