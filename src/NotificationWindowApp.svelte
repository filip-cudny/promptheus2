<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import NotificationStack from "$lib/components/notification-window/NotificationStack.svelte";
  import { init, destroy } from "$lib/stores/notificationWindow.svelte";

  onMount(() => {
    init();
  });

  onDestroy(() => {
    destroy();
  });
</script>

<NotificationStack />
