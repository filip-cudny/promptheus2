<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let greeting = $state("");
  let name = $state("");

  async function greet() {
    greeting = await invoke("greet", { name });
  }
</script>

<main>
  <h1>Promptheus</h1>

  <form onsubmit={greet}>
    <input bind:value={name} placeholder="Enter a name..." />
    <button type="submit">Greet</button>
  </form>

  <p>{greeting}</p>
</main>
