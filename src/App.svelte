<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { emit } from "@tauri-apps/api/event";
  import { init, destroy } from "$lib/stores/notifications.svelte";
  import NotificationToast from "$lib/components/ui/NotificationToast.svelte";

  let greeting = $state("");
  let name = $state("");

  onMount(() => {
    init();
  });

  onDestroy(() => {
    destroy();
  });

  async function greet() {
    greeting = await invoke("greet", { name });
  }

  async function openContextMenu() {
    await emit("show-context-menu");
  }
</script>

<main>
  <h1>Promptheus</h1>

  <form onsubmit={greet}>
    <input bind:value={name} placeholder="Enter a name..." />
    <button type="submit">Greet</button>
  </form>

  <p>{greeting}</p>

  <button onclick={openContextMenu}>Open Context Menu</button>
</main>

<NotificationToast />
