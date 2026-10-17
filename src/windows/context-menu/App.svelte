<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import ContextMenu from "$lib/components/features/context-menu/ContextMenu.svelte";
  import { initLogo } from "$lib/stores/logo.svelte";
  import * as executionStore from "$lib/stores/execution.svelte";

  onMount(() => {
    initLogo();
    executionStore.init();
  });

  onDestroy(() => {
    executionStore.destroy();
  });
</script>

<ContextMenu />

<style>
  :global(html),
  :global(body) {
    background: transparent;
  }
</style>
