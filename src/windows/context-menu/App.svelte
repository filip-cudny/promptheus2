<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import ContextMenu from "$lib/components/context-menu/ContextMenu.svelte";
  import { getContextStore } from "$lib/stores/context.svelte";
  import * as executionStore from "$lib/stores/execution.svelte";

  const contextStore = getContextStore();

  onMount(() => {
    contextStore.init();
    executionStore.init();
  });

  onDestroy(() => {
    contextStore.destroy();
    executionStore.destroy();
  });
</script>

<ContextMenu />
