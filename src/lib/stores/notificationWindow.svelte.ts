import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { debug, error as logError } from "@tauri-apps/plugin-log";
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

const NOTIFICATION_HEIGHT = 64;
const NOTIFICATION_GAP = 8;

let notifications = $state<ActiveNotification[]>([]);
let unlisten: (() => void) | null = null;

function add(level: NotificationLevel, title: string, message?: string) {
  const duration = DEFAULT_DURATIONS[level];
  const notification: ActiveNotification = {
    id: crypto.randomUUID(),
    level,
    title,
    message,
    created_at: Date.now(),
    duration,
  };

  notifications.push(notification);
  debug(
    `[notification-window] added: level=${level} title="${title}" count=${notifications.length}`,
  );
  updateWindow(notifications.length);

  setTimeout(() => {
    remove(notification.id);
  }, duration);
}

function remove(id: string) {
  const index = notifications.findIndex((n) => n.id === id);
  if (index !== -1) {
    notifications.splice(index, 1);
    debug(
      `[notification-window] removed: id=${id} remaining=${notifications.length}`,
    );
    const fadeOutDelay = notifications.length === 0 ? 350 : 0;
    setTimeout(() => {
      updateWindow(notifications.length);
    }, fadeOutDelay);
  }
}

function updateWindow(count: number) {
  const height =
    count > 0
      ? count * NOTIFICATION_HEIGHT + (count - 1) * NOTIFICATION_GAP + 20
      : 0;
  debug(
    `[notification-window] updateWindow: count=${count} height=${Math.ceil(height)}`,
  );
  invoke("update_notification_window", {
    count,
    height: Math.ceil(height),
  }).catch((e) => {
    logError(`[notification-window] update_notification_window failed: ${e}`);
  });
}

async function init() {
  debug("[notification-window] init: registering event listener");
  unlisten = await listen<NotificationPayload>("notification", (event) => {
    debug(
      `[notification-window] received event: ${JSON.stringify(event.payload)}`,
    );
    const { level, title, message } = event.payload;
    add(level, title, message);
  });
  debug("[notification-window] init: listener registered");
}

function destroy() {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
}

export { notifications, remove, init, destroy };
