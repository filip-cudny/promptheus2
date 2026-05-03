import { invoke } from "@tauri-apps/api/core";
import { tick } from "svelte";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalPosition, LogicalSize } from "@tauri-apps/api/dpi";
import { debug as logDebug } from "@tauri-apps/plugin-log";

interface WorkArea {
  cursorX: number;
  cursorY: number;
  workX: number;
  workY: number;
  workWidth: number;
  workHeight: number;
}

const MENU_WIDTH = 320;

type Opts = {
  getMenuEl: () => HTMLDivElement | undefined;
  isVisible: () => boolean;
  getWorkArea: () => WorkArea | null;
};

export function useMenuPositioning(opts: Opts) {
  let hoverEnabled = $state(false);
  let resizeGeneration = 0;

  function getSkillsAnchorOffset(): number {
    const menuEl = opts.getMenuEl();
    if (!menuEl) return 0;
    const anchor = menuEl.querySelector("[data-section='skills-anchor']");
    if (!anchor) return 0;
    return (anchor as HTMLElement).offsetTop;
  }

  async function triggerReposition() {
    const gen = ++resizeGeneration;
    await tick();
    if (gen !== resizeGeneration) return;
    const menuEl = opts.getMenuEl();
    if (!menuEl || !opts.isVisible()) return;

    let height = menuEl.scrollHeight + 2;
    const win = getCurrentWebviewWindow();
    const wa = opts.getWorkArea();
    let x = 0, y = 0;

    function positionFromHeight(h: number) {
      if (!wa) return;
      const anchorOffset = getSkillsAnchorOffset();
      x = wa.cursorX;
      y = wa.cursorY - anchorOffset;
      const rightEdge = wa.workX + wa.workWidth;
      const bottomEdge = wa.workY + wa.workHeight;
      if (x + MENU_WIDTH > rightEdge) x = rightEdge - MENU_WIDTH;
      if (y + h > bottomEdge) y = bottomEdge - h;
      if (x < wa.workX) x = wa.workX;
      if (y < wa.workY) y = wa.workY;
    }

    positionFromHeight(height);
    hoverEnabled = false;
    await win.setSize(new LogicalSize(MENU_WIDTH, height));
    if (gen !== resizeGeneration || !opts.isVisible()) return;
    if (wa) {
      await win.setPosition(new LogicalPosition(x, y));
      if (gen !== resizeGeneration || !opts.isVisible()) return;
    }
    await invoke("show_context_menu_panel");
    if (gen !== resizeGeneration || !opts.isVisible()) return;

    const correctedHeight = menuEl.scrollHeight + 2;
    if (correctedHeight !== height) {
      height = correctedHeight;
      positionFromHeight(height);
      await win.setSize(new LogicalSize(MENU_WIDTH, height));
      if (gen !== resizeGeneration || !opts.isVisible()) return;
      if (wa) {
        await win.setPosition(new LogicalPosition(x, y));
        if (gen !== resizeGeneration || !opts.isVisible()) return;
      }
    }

    await invoke("focus_context_menu");
    logDebug(`[ctx-menu] opened at (${x}, ${y}), size ${MENU_WIDTH}x${height}`);
  }

  return {
    get hoverEnabled() { return hoverEnabled; },
    enableHover() {
      if (!hoverEnabled) hoverEnabled = true;
    },
    triggerReposition,
  };
}

export type MenuPositioning = ReturnType<typeof useMenuPositioning>;
