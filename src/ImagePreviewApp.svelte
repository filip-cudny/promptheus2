<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { LogicalSize, LogicalPosition } from "@tauri-apps/api/dpi";

  interface WorkArea {
    cursorX: number;
    cursorY: number;
    workX: number;
    workY: number;
    workWidth: number;
    workHeight: number;
  }

  const MAX_SIZE = 800;
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

    let width = img.naturalWidth;
    let height = img.naturalHeight;
    const longest = Math.max(width, height);
    if (longest > MAX_SIZE) {
      const scale = MAX_SIZE / longest;
      width = Math.round(width * scale);
      height = Math.round(height * scale);
    }

    await win.setSize(new LogicalSize(width, height));

    try {
      const wa = await invoke<WorkArea>("get_image_preview_work_area");
      const rightEdge = wa.workX + wa.workWidth;
      const bottomEdge = wa.workY + wa.workHeight;

      let x = wa.cursorX;
      let y = wa.cursorY;
      if (x + width > rightEdge) x = rightEdge - width;
      if (y + height > bottomEdge) y = bottomEdge - height;
      if (x < wa.workX) x = wa.workX;
      if (y < wa.workY) y = wa.workY;

      await win.setPosition(new LogicalPosition(x, y));
    } catch {
      // fallback: no repositioning
    }

    await win.show();
    await win.setFocus();
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
      invoke("hide_dialog_window", { label: "image-preview" });
    }, ANIM_MS);
  }

  onMount(() => {
    const unlistenLoad = win.listen("load-image", () => {
      loadImage();
    });

    const unlistenFocus = win.onFocusChanged(({ payload: focused }) => {
      if (!focused) hide();
    });

    return () => {
      unlistenLoad.then((fn) => fn());
      unlistenFocus.then((fn) => fn());
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
