import { invoke } from "@tauri-apps/api/core";
import { debug as logDebug, error as logError, warn as logWarn } from "@tauri-apps/plugin-log";
import { toast } from "svelte-sonner";

type Level = "success" | "error" | "info" | "warning";

type Payload = {
  id: string;
  level: Level;
  title: string;
  message: string | null;
  monochromatic: boolean;
};

const DURATIONS: Record<Level, number> = {
  success: 2000,
  error: 4000,
  info: 2000,
  warning: 3000,
};

const WATCHDOG_GRACE_MS = 1500;

type LiveToast = {
  timer: number;
  shownAt: number;
  plannedMs: number;
  deadline: number;
};

type CloseSource = "autoClose" | "dismiss" | "watchdog";
type WatchdogSource = "timer" | "sweep";

export type ToasterHandlers = {
  onmouseenter: () => void;
  onmouseleave: () => void;
  onpointerdown: (e: PointerEvent) => void;
  onpointerup: (e: PointerEvent) => void;
};

export function useNotificationDrain() {
  let active = 0;
  let pointerOverToaster = false;
  const live = new Map<string, LiveToast>();
  let syncTimer: number | null = null;
  let ro: ResizeObserver | null = null;

  function measureStack(toaster: HTMLElement): number {
    const toasts = toaster.querySelectorAll<HTMLElement>("[data-sonner-toast]");
    if (toasts.length === 0) return 0;
    let total = 0;
    for (const t of toasts) total += t.offsetHeight;
    total += Math.max(0, toasts.length - 1) * 14;
    return total;
  }

  function syncWindow() {
    const el = document.querySelector<HTMLElement>("[data-sonner-toaster]");
    const stackHeight = el ? measureStack(el) : 0;
    const height = active > 0 ? stackHeight + 20 : 0;
    invoke("update_notification_window", { count: active, height }).catch((e) => {
      logError(`update_notification_window failed: ${e}`);
    });
  }

  function scheduleSync() {
    if (syncTimer !== null) return;
    syncTimer = window.setTimeout(() => {
      syncTimer = null;
      syncWindow();
    }, 30);
  }

  function closeToast(id: string, path: CloseSource) {
    const entry = live.get(id);
    if (!entry || !live.delete(id)) return;
    clearTimeout(entry.timer);
    active = Math.max(0, active - 1);
    logDebug(`toast closed id=${id} path=${path} elapsedMs=${Date.now() - entry.shownAt}`);
    scheduleSync();
  }

  function watchdog(id: string, source: WatchdogSource) {
    const entry = live.get(id);
    if (!entry) return;
    const expanded =
      document
        .querySelector("[data-sonner-toaster] [data-sonner-toast]")
        ?.getAttribute("data-expanded") ?? "none";
    logWarn(
      `toast watchdog id=${id} source=${source} plannedMs=${entry.plannedMs} actualMs=${Date.now() - entry.shownAt} visibility=${document.visibilityState} hasFocus=${document.hasFocus()} expanded=${expanded} pointerOverToaster=${pointerOverToaster}`,
    );
    closeToast(id, "watchdog");
    toast.dismiss(id);
  }

  function sweepExpired() {
    const now = Date.now();
    for (const [id, entry] of [...live]) {
      if (now >= entry.deadline) watchdog(id, "sweep");
    }
  }

  async function drainPending() {
    sweepExpired();
    try {
      const pending = await invoke<Payload[]>("drain_pending_notifications");
      if (pending.length === 0) return;
      logDebug(`draining ${pending.length} pending notification(s)`);
      for (const n of pending) {
        active++;
        const plannedMs = DURATIONS[n.level] + WATCHDOG_GRACE_MS;
        const shownAt = Date.now();
        live.set(n.id, {
          timer: window.setTimeout(() => watchdog(n.id, "timer"), plannedMs),
          shownAt,
          plannedMs,
          deadline: shownAt + plannedMs,
        });
        const klass = n.monochromatic ? "p-toast p-mono" : `p-toast p-color-${n.level}`;
        logDebug(`toast shown id=${n.id} level=${n.level} duration=${DURATIONS[n.level]}`);
        toast[n.level](n.title, {
          id: n.id,
          description: n.message ?? undefined,
          duration: DURATIONS[n.level],
          class: klass,
          onAutoClose: () => closeToast(n.id, "autoClose"),
          onDismiss: () => closeToast(n.id, "dismiss"),
        });
      }
      scheduleSync();
    } catch (e) {
      logError(`drain_pending_notifications failed: ${e}`);
    }
  }

  const toasterHandlers: ToasterHandlers = {
    onmouseenter: () => {
      pointerOverToaster = true;
      logDebug("toaster mouseenter");
    },
    onmouseleave: () => {
      pointerOverToaster = false;
      logDebug("toaster mouseleave");
    },
    onpointerdown: (e) => logDebug(`toaster pointerdown button=${e.button}`),
    onpointerup: (e) => logDebug(`toaster pointerup button=${e.button}`),
  };

  const onWindowFocus = () => logDebug("window focus");
  const onWindowBlur = () => logDebug("window blur");
  const onWindowKeydown = (e: KeyboardEvent) =>
    logDebug(`window keydown altKey=${e.altKey} isKeyT=${e.code === "KeyT"}`);
  const onVisibilityChange = () => logDebug(`visibilitychange state=${document.visibilityState}`);

  function init() {
    window.addEventListener("focus", onWindowFocus);
    window.addEventListener("blur", onWindowBlur);
    window.addEventListener("keydown", onWindowKeydown);
    document.addEventListener("visibilitychange", onVisibilityChange);
    const attachObserver = () => {
      const el = document.querySelector<HTMLElement>("[data-sonner-toaster]");
      if (!el) {
        requestAnimationFrame(attachObserver);
        return;
      }
      ro = new ResizeObserver(() => scheduleSync());
      ro.observe(el);
    };
    attachObserver();
    drainPending();
  }

  function destroy() {
    window.removeEventListener("focus", onWindowFocus);
    window.removeEventListener("blur", onWindowBlur);
    window.removeEventListener("keydown", onWindowKeydown);
    document.removeEventListener("visibilitychange", onVisibilityChange);
    for (const entry of live.values()) clearTimeout(entry.timer);
    live.clear();
    if (ro) {
      ro.disconnect();
      ro = null;
    }
    if (syncTimer !== null) {
      clearTimeout(syncTimer);
      syncTimer = null;
    }
  }

  return { init, destroy, drainPending, toasterHandlers };
}

export type NotificationDrain = ReturnType<typeof useNotificationDrain>;
