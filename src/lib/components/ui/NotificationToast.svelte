<script lang="ts">
  import { notifications, remove } from "$lib/stores/notifications.svelte";
  import type { NotificationLevel } from "$lib/types";

  const OPACITY = 0.8;

  function toastIn(node: HTMLElement) {
    return {
      duration: 300,
      css: (t: number) => `
        opacity: ${t * OPACITY};
        transform: translateY(${(1 - t) * 10}px);
      `,
    };
  }

  function toastOut(node: HTMLElement) {
    return {
      duration: 300,
      css: (t: number) => `
        opacity: ${t * OPACITY};
        transform: translateY(${(1 - t) * 10}px);
      `,
    };
  }

  const ICON_COLORS: Record<NotificationLevel, string> = {
    success: "#43803e",
    error: "#c94a4a",
    info: "#6A7D93",
    warning: "#b8860b",
  };

  const ICONS: Record<NotificationLevel, string> = {
    success: "✓",
    error: "✕",
    info: "ℹ",
    warning: "⚠",
  };
</script>

<div class="toast-container">
  {#each notifications as notification (notification.id)}
    <div
      class="toast toast-{notification.level}"
      in:toastIn
      out:toastOut
      onoutroend={() => remove(notification.id)}
    >
      <span class="toast-icon" style="color: {ICON_COLORS[notification.level]}">
        {ICONS[notification.level]}
      </span>
      <div class="toast-content">
        <span class="toast-title">{notification.title}</span>
        {#if notification.message}
          <span class="toast-message">{notification.message}</span>
        {/if}
      </div>
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    bottom: 20px;
    right: 20px;
    z-index: 9999;
    display: flex;
    flex-direction: column-reverse;
    gap: 8px;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 16px;
    border-radius: 8px;
    background: #ffffff;
    border: 1px solid rgba(200, 200, 200, 0.7);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    pointer-events: auto;
    max-width: 360px;
    min-width: 240px;
  }

  .toast-icon {
    font-size: 16px;
    line-height: 1;
    flex-shrink: 0;
    margin-top: 1px;
  }

  .toast-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .toast-title {
    font-weight: 600;
    font-size: 14px;
    color: #1a1a1a;
  }

  .toast-message {
    font-size: 13px;
    color: #4a4a4a;
  }
</style>
