<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { LogicalSize } from "@tauri-apps/api/dpi";

  const MAX_SIZE = 400;

  let src = $state("");
  const win = getCurrentWebviewWindow();

  async function loadImage() {
    const payload = await invoke<{
      data: string;
      media_type: string;
    } | null>("get_pending_image");
    if (!payload) return;

    const dataUri = `data:${payload.media_type};base64,${payload.data}`;
    const img = new Image();
    img.src = dataUri;
    await img.decode();

    const ratio = img.naturalWidth / img.naturalHeight;
    const width = ratio >= 1 ? MAX_SIZE : Math.round(MAX_SIZE * ratio);
    const height = ratio >= 1 ? Math.round(MAX_SIZE / ratio) : MAX_SIZE;

    await win.setSize(new LogicalSize(width, height));
    src = dataUri;
  }

  function hide() {
    src = "";
    win.hide();
  }

  onMount(() => {
    const unlistenLoad = win.listen("load-image", () => {
      loadImage();
    });

    const unlistenFocus = win.onFocusChanged(({ payload: focused }) => {
      if (!focused) hide();
    });

    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") hide();
    };
    window.addEventListener("keydown", handleKey);

    return () => {
      unlistenLoad.then((fn) => fn());
      unlistenFocus.then((fn) => fn());
      window.removeEventListener("keydown", handleKey);
    };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="preview" onclick={hide} onkeydown={() => {}}>
  {#if src}
    <img {src} alt="Preview" class="preview-image" />
  {/if}
</div>

<style>
  .preview {
    width: 100%;
    height: 100%;
    cursor: pointer;
  }

  .preview-image {
    width: 100%;
    height: 100%;
    object-fit: contain;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.15);
  }
</style>
