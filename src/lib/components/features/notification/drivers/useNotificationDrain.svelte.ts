import { invoke } from "@tauri-apps/api/core";
import { debug as logDebug, error as logError } from "@tauri-apps/plugin-log";
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

export function useNotificationDrain() {
  let active = 0;
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

  async function drainPending() {
    try {
      const pending = await invoke<Payload[]>("drain_pending_notifications");
      if (pending.length === 0) return;
      logDebug(`draining ${pending.length} pending notification(s)`);
      const onToastGone = () => {
        active = Math.max(0, active - 1);
        scheduleSync();
      };
      for (const n of pending) {
        active++;
        const klass = n.monochromatic ? "p-toast p-mono" : `p-toast p-color-${n.level}`;
        toast[n.level](n.title, {
          description: n.message ?? undefined,
          duration: DURATIONS[n.level],
          class: klass,
          onAutoClose: onToastGone,
          onDismiss: onToastGone,
        });
      }
      scheduleSync();
    } catch (e) {
      logError(`drain_pending_notifications failed: ${e}`);
    }
  }

  function init() {
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
    if (ro) {
      ro.disconnect();
      ro = null;
    }
    if (syncTimer !== null) {
      clearTimeout(syncTimer);
      syncTimer = null;
    }
  }

  return { init, destroy, drainPending };
}

export type NotificationDrain = ReturnType<typeof useNotificationDrain>;
