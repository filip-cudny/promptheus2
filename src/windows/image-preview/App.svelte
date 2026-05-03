<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import ImagePreview from "$lib/components/features/image-preview/ImagePreview.svelte";
  import { useImagePreviewWindow } from "$lib/components/features/image-preview/drivers/useImagePreviewWindow.svelte";

  const preview = useImagePreviewWindow();

  onMount(() => preview.init());
  onDestroy(() => preview.destroy());
</script>

<ImagePreview src={preview.src} visible={preview.visible} onHide={preview.hide} />
