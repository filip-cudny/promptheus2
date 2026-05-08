import { getCurrentWindow } from "@tauri-apps/api/window";

const INSET = 5;
const STYLE_ELEMENT_ID = "edge-resize-cursor-override";

export type EdgeResizeRole = "toolbar" | "content";

function isLinux(): boolean {
  if (typeof navigator === "undefined") return false;
  const platform = navigator.platform || "";
  const ua = navigator.userAgent || "";
  return /Linux/i.test(platform) && !/Mac|Win/i.test(platform) && !/Android/i.test(ua);
}

function ensureStyleEl(): HTMLStyleElement {
  let el = document.getElementById(STYLE_ELEMENT_ID) as HTMLStyleElement | null;
  if (!el) {
    el = document.createElement("style");
    el.id = STYLE_ELEMENT_ID;
    document.head.appendChild(el);
  }
  return el;
}

function setOverrideCursor(styleEl: HTMLStyleElement, name: string): void {
  const next = name
    ? `*, *::before, *::after { cursor: ${name} !important; }`
    : "";
  if (styleEl.textContent !== next) {
    styleEl.textContent = next;
  }
}

function cursorForEdges(left: boolean, right: boolean, top: boolean, bottom: boolean): string {
  if (left && top) return "nw-resize";
  if (right && top) return "ne-resize";
  if (left && bottom) return "sw-resize";
  if (right && bottom) return "se-resize";
  if (left) return "w-resize";
  if (right) return "e-resize";
  if (top) return "n-resize";
  if (bottom) return "s-resize";
  return "";
}

export function attachEdgeResizeCursor(role: EdgeResizeRole): () => void {
  if (!isLinux()) return () => {};

  const styleEl = ensureStyleEl();
  let maximized = false;

  const win = getCurrentWindow();

  const refreshMaximized = () => {
    win
      .isMaximized()
      .then((v) => {
        maximized = v;
        if (maximized) setOverrideCursor(styleEl, "");
      })
      .catch(() => {});
  };
  refreshMaximized();

  let unlistenResized: (() => void) | null = null;
  win
    .onResized(() => refreshMaximized())
    .then((un) => {
      unlistenResized = un;
    })
    .catch(() => {});

  const onMove = (e: MouseEvent) => {
    if (maximized) {
      setOverrideCursor(styleEl, "");
      return;
    }
    const w = window.innerWidth;
    const h = window.innerHeight;
    const left = e.clientX < INSET;
    const right = e.clientX >= w - INSET;
    const top = role === "toolbar" && e.clientY < INSET;
    const bottom = role === "content" && e.clientY >= h - INSET;
    setOverrideCursor(styleEl, cursorForEdges(left, right, top, bottom));
  };

  const clear = () => setOverrideCursor(styleEl, "");

  window.addEventListener("mousemove", onMove);
  document.addEventListener("mouseleave", clear);
  window.addEventListener("blur", clear);

  return () => {
    window.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseleave", clear);
    window.removeEventListener("blur", clear);
    if (unlistenResized) unlistenResized();
    setOverrideCursor(styleEl, "");
  };
}
