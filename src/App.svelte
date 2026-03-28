<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { setupHotkeyListener } from "$lib/services/hotkeys";

  let greeting = $state("");
  let name = $state("");
  let unlistenHotkeys: (() => void) | undefined;

  onMount(() => {
    setupHotkeyListener().then((unlisten) => {
      unlistenHotkeys = unlisten;
    });

    return () => {
      unlistenHotkeys?.();
    };
  });

  async function greet() {
    greeting = await invoke("greet", { name });
  }

  async function openContextMenu() {
    await invoke("show_context_menu_window");
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

