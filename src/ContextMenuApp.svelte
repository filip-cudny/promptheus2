<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import ContextMenu from "$lib/components/context-menu/ContextMenu.svelte";
  import { getContextStore } from "$lib/stores/context.svelte";

  const contextStore = getContextStore();

  onMount(() => {
    contextStore.init();
  });

  onDestroy(() => {
    contextStore.destroy();
  });
</script>

<ContextMenu />
