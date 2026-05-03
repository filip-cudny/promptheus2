<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { attachConsole } from "@tauri-apps/plugin-log";
  import NotificationStack from "$lib/components/features/notification/NotificationStack.svelte";
  import { useNotificationDrain } from "$lib/components/features/notification/drivers/useNotificationDrain.svelte";

  const isLinux = navigator.userAgent.includes("Linux");
  const drain = useNotificationDrain();

  onMount(() => {
    attachConsole();
    if (isLinux) document.documentElement.classList.add("linux");
    (window as unknown as { drainPending: () => void }).drainPending = drain.drainPending;
    drain.init();
  });

  onDestroy(() => drain.destroy());
</script>

<NotificationStack expand={isLinux} />

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
</style>
