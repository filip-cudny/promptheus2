<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import Palette from "$lib/components/features/palette/Palette.svelte";
  import { usePaletteIpc } from "$lib/components/features/palette/drivers/usePaletteIpc.svelte";

  let inputEl: HTMLInputElement | undefined = $state();

  const ipc = usePaletteIpc({
    onShown: () => inputEl?.focus(),
  });

  onMount(() => ipc.init());
  onDestroy(() => ipc.destroy());
</script>

<Palette
  visible={ipc.visible}
  activeId={ipc.activeId}
  webviewProviders={ipc.webviewProviders}
  bind:inputRef={inputEl}
  onDismiss={(selectedId) => ipc.dismiss(selectedId)}
  onReloadActive={() => ipc.reloadActive()}
/>

<style>
  :global(html),
  :global(body) {
    background: transparent;
    margin: var(--space-0);
  }
</style>
