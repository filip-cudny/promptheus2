<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { attachConsole, debug as logDebug, error as logError } from "@tauri-apps/plugin-log";
  import { Toaster, toast } from "svelte-sonner";
  import { CircleCheck, CircleX, Info, CircleAlert } from "lucide-svelte";

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

  let active = 0;
  let syncTimer: number | null = null;

  function scheduleSync() {
    if (syncTimer !== null) return;
    syncTimer = window.setTimeout(() => {
      syncTimer = null;
      syncWindow();
    }, 30);
  }

  function syncWindow() {
    const el = document.querySelector<HTMLElement>("[data-sonner-toaster]");
    const stackHeight = el ? measureStack(el) : 0;
    const height = active > 0 ? stackHeight + 20 : 0;
    invoke("update_notification_window", { count: active, height }).catch((e) => {
      logError(`update_notification_window failed: ${e}`);
    });
  }

  function measureStack(toaster: HTMLElement): number {
    const toasts = toaster.querySelectorAll<HTMLElement>("[data-sonner-toast]");
    if (toasts.length === 0) return 0;
    let total = 0;
    for (const t of toasts) {
      total += t.offsetHeight;
    }
    total += Math.max(0, toasts.length - 1) * 14;
    return total;
  }

  async function drainPending() {
    try {
      const pending = await invoke<Payload[]>("drain_pending_notifications");
      if (pending.length === 0) return;
      logDebug(`draining ${pending.length} pending notification(s)`);
      for (const n of pending) {
        active++;
        const klass = n.monochromatic ? "p-toast p-mono" : `p-toast p-color-${n.level}`;
        toast[n.level](n.title, {
          description: n.message ?? undefined,
          duration: DURATIONS[n.level],
          class: klass,
          onAutoClose: () => {
            active = Math.max(0, active - 1);
            scheduleSync();
          },
          onDismiss: () => {
            active = Math.max(0, active - 1);
            scheduleSync();
          },
        });
      }
      scheduleSync();
    } catch (e) {
      logError(`drain_pending_notifications failed: ${e}`);
    }
  }

  onMount(() => {
    attachConsole();
    (window as unknown as { drainPending: () => void }).drainPending = drainPending;

    let ro: ResizeObserver | null = null;
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

    return () => {
      if (ro) ro.disconnect();
      if (syncTimer !== null) clearTimeout(syncTimer);
    };
  });
</script>

<Toaster position="bottom-right" closeButton={false} richColors={false} duration={2000}>
  {#snippet successIcon()}
    <CircleCheck size={20} strokeWidth={2} />
  {/snippet}
  {#snippet errorIcon()}
    <CircleX size={20} strokeWidth={2} />
  {/snippet}
  {#snippet infoIcon()}
    <Info size={20} strokeWidth={2} />
  {/snippet}
  {#snippet warningIcon()}
    <CircleAlert size={20} strokeWidth={2} />
  {/snippet}
</Toaster>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    padding: 0;
    background: transparent;
    overflow: hidden;
    width: 100%;
    height: 100%;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }

  :global([data-sonner-toaster]) {
    --width: 360px;
    --gap: 14px;
    --border-radius: 8px;
    --offset-right: 20px;
    --offset-bottom: 20px;
    --offset-top: 20px;
    --offset-left: 20px;
    --normal-bg: #ffffff;
    --normal-border: rgba(200, 200, 200, 0.9);
    --normal-text: #1a1a1a;
    --success-bg: #ffffff;
    --success-border: rgba(200, 200, 200, 0.9);
    --success-text: #1a1a1a;
    --error-bg: #ffffff;
    --error-border: rgba(200, 200, 200, 0.9);
    --error-text: #1a1a1a;
    --info-bg: #ffffff;
    --info-border: rgba(200, 200, 200, 0.9);
    --info-text: #1a1a1a;
    --warning-bg: #ffffff;
    --warning-border: rgba(200, 200, 200, 0.9);
    --warning-text: #1a1a1a;
  }

  :global([data-sonner-toast]) {
    background: #ffffff !important;
    border: 1px solid rgba(200, 200, 200, 0.9) !important;
    box-shadow: none !important;
    padding: 12px 16px !important;
  }

  :global([data-sonner-toast] [data-title]) {
    color: #1a1a1a;
    font-weight: 600;
    font-size: 14px;
  }

  :global([data-sonner-toast] [data-description]) {
    color: #4a4a4a;
    font-size: 13px;
  }

  :global([data-sonner-toast] [data-icon]) {
    width: 20px;
    height: 20px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  :global(.p-mono [data-icon]) {
    color: #1a1a1a;
  }
  :global(.p-color-success [data-icon]) {
    color: #43803e;
  }
  :global(.p-color-error [data-icon]) {
    color: #c94a4a;
  }
  :global(.p-color-info [data-icon]) {
    color: #6a7d93;
  }
  :global(.p-color-warning [data-icon]) {
    color: #b8860b;
  }
</style>
