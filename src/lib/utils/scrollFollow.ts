export type ScrollMode = "follow" | "pinned" | "free";

export interface ScrollState {
  mode: ScrollMode;
  spacer: boolean;
}

export type ScrollEvent =
  | { type: "pin" }
  | { type: "reset" }
  | { type: "contentChange" }
  | { type: "userScroll"; atBottom: boolean }
  | { type: "programmaticScroll" };

export type ScrollAction = "none" | "toBottom" | "toPinned";

export const INITIAL_SCROLL_STATE: ScrollState = { mode: "follow", spacer: false };

const BOTTOM_TOLERANCE_PX = 4;
const GESTURE_GRACE_MS = 250;

export function reduceScroll(
  state: ScrollState,
  event: ScrollEvent,
): { state: ScrollState; action: ScrollAction } {
  switch (event.type) {
    case "pin":
      return { state: { mode: "pinned", spacer: true }, action: "toPinned" };
    case "reset":
      return { state: { mode: "follow", spacer: false }, action: "toBottom" };
    case "contentChange":
      return { state, action: state.mode === "follow" ? "toBottom" : "none" };
    case "userScroll":
      return { state: { ...state, mode: event.atBottom ? "follow" : "free" }, action: "none" };
    case "programmaticScroll":
      return { state, action: "none" };
  }
}

function isAtBottom(el: HTMLElement): boolean {
  return el.scrollHeight - el.scrollTop - el.clientHeight <= BOTTOM_TOLERANCE_PX;
}

function scrollToPinned(el: HTMLElement, target: HTMLElement) {
  const paddingTop = parseFloat(getComputedStyle(el).paddingTop) || 0;
  const contentTop = el.getBoundingClientRect().top + el.clientTop + paddingTop;
  el.scrollTop += target.getBoundingClientRect().top - contentTop;
}

export function createScrollFollower(opts: {
  getContainer: () => HTMLElement | undefined;
  getPinnedTarget: () => HTMLElement | null | undefined;
  onChange: (state: ScrollState) => void;
}) {
  let state = INITIAL_SCROLL_STATE;
  let gestureUntil = 0;
  let pointerHeld = false;

  function perform(action: ScrollAction) {
    if (action === "none") return;
    requestAnimationFrame(() => {
      const el = opts.getContainer();
      if (!el) return;
      gestureUntil = 0;
      pointerHeld = false;
      if (action === "toBottom") {
        el.scrollTop = el.scrollHeight;
        return;
      }
      const target = opts.getPinnedTarget();
      if (target) scrollToPinned(el, target);
    });
  }

  function dispatch(event: ScrollEvent) {
    const next = reduceScroll(state, event);
    const changed = next.state.mode !== state.mode || next.state.spacer !== state.spacer;
    state = next.state;
    if (changed) opts.onChange(state);
    perform(next.action);
  }

  function markGesture() {
    gestureUntil = performance.now() + GESTURE_GRACE_MS;
  }

  function onPointerdown() {
    pointerHeld = true;
    markGesture();
    const release = () => {
      pointerHeld = false;
      markGesture();
      window.removeEventListener("pointerup", release);
      window.removeEventListener("pointercancel", release);
    };
    window.addEventListener("pointerup", release);
    window.addEventListener("pointercancel", release);
  }

  function onScroll() {
    const el = opts.getContainer();
    if (!el) return;
    const byUser = pointerHeld || performance.now() < gestureUntil;
    dispatch(byUser ? { type: "userScroll", atBottom: isAtBottom(el) } : { type: "programmaticScroll" });
  }

  return {
    dispatch,
    onScroll,
    onWheel: markGesture,
    onKeydown: markGesture,
    onPointerdown,
  };
}
