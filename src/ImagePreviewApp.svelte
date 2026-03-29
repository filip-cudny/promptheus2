<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { LogicalSize } from "@tauri-apps/api/dpi";

  const MAX_SIZE = 400;
  const ANIM_MS = 150;

  let src = $state("");
  let visible = $state(false);
  let hiding = false;
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
    requestAnimationFrame(() => (visible = true));
  }

  function hide() {
    if (hiding) return;
    hiding = true;
    visible = false;
    setTimeout(() => {
      src = "";
      hiding = false;
      win.hide();
    }, ANIM_MS);
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
    <img {src} alt="Preview" class="preview-image" class:visible />
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
    opacity: 0;
    transform: scale(0.92);
    transition:
      opacity 150ms ease-out,
      transform 150ms ease-out;
  }

  .preview-image.visible {
    opacity: 1;
    transform: scale(1);
  }
</style>
