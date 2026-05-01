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

  const isLinux = navigator.userAgent.includes("Linux");

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

  onMount(() => {
    attachConsole();
    (window as unknown as { drainPending: () => void }).drainPending = drainPending;

    if (isLinux) {
      document.documentElement.classList.add("linux");
    }

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

<Toaster position="bottom-right" closeButton={false} richColors={false} duration={2000} expand={isLinux} gap={14}>
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
    margin: var(--space-0);
    padding: var(--space-0);
    background: transparent;
    overflow: hidden;
    width: 100%;
    height: 100%;
    font-family: var(--font-sans);
  }

  :global([data-sonner-toaster]) {
    --border-radius: 8px;
    --offset-right: 0px;
    --offset-bottom: 0px;
    --offset-top: 0px;
    --offset-left: 0px;
    inset: 0 !important;
    width: auto !important;
    height: auto !important;
  }

  :global([data-sonner-toast]) {
    width: 300px !important;
    background: var(--notification-bg) !important;
    border: 1px solid rgba(200, 200, 200, 0.9) !important;
    box-shadow: none !important;
    padding: var(--space-6) var(--space-8) !important;
  }

  :global([data-sonner-toast][data-y-position="bottom"][data-x-position="right"]) {
    bottom: 20px !important;
    right: 20px !important;
    left: auto !important;
  }

  :global(html.linux [data-sonner-toast]),
  :global(html.linux [data-sonner-toast] *) {
    transition: none !important;
    animation: none !important;
  }

  :global([data-sonner-toast] [data-title]) {
    color: var(--notification-fg) !important;
    font-weight: var(--font-weight-semibold) !important;
    font-size: var(--font-size-lg) !important;
  }

  :global([data-sonner-toast] [data-description]) {
    color: var(--text-secondary) !important;
    font-size: var(--font-size-base) !important;
  }

  :global([data-sonner-toast] [data-icon]) {
    width: 20px;
    height: 20px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  :global(.p-mono [data-icon]) {
    color: var(--notification-fg);
  }
  :global(.p-color-success [data-icon]) {
    color: var(--success);
  }
  :global(.p-color-error [data-icon]) {
    color: var(--danger);
  }
  :global(.p-color-info [data-icon]) {
    color: #6a7d93;
  }
  :global(.p-color-warning [data-icon]) {
    color: var(--warning);
  }
</style>
