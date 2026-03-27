import { listen } from "@tauri-apps/api/event";
import type {
  ActiveNotification,
  NotificationLevel,
  NotificationPayload,
} from "$lib/types";

const DEFAULT_DURATIONS: Record<NotificationLevel, number> = {
  success: 2000,
  error: 4000,
  info: 2000,
  warning: 3000,
};

let notifications = $state<ActiveNotification[]>([]);
let unlisten: (() => void) | null = null;

function add(
  level: NotificationLevel,
  title: string,
  message?: string,
  duration?: number,
) {
  const resolvedDuration = duration ?? DEFAULT_DURATIONS[level];
  const notification: ActiveNotification = {
    id: crypto.randomUUID(),
    level,
    title,
    message,
    created_at: Date.now(),
    duration: resolvedDuration,
  };

  notifications.push(notification);

  setTimeout(() => {
    remove(notification.id);
  }, resolvedDuration);
}

function remove(id: string) {
  notifications = notifications.filter((n) => n.id !== id);
}

async function init() {
  unlisten = await listen<NotificationPayload>("notification", (event) => {
    const { level, title, message } = event.payload;
    add(level, title, message);
  });
}

function destroy() {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
}

export { notifications, add, remove, init, destroy };
