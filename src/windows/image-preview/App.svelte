<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { LogicalSize, LogicalPosition } from "@tauri-apps/api/dpi";

  interface WorkArea {
    cursor_x: number;
    cursor_y: number;
    work_x: number;
    work_y: number;
    work_width: number;
    work_height: number;
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
      const rightEdge = wa.work_x + wa.work_width;
      const bottomEdge = wa.work_y + wa.work_height;

      let x = wa.cursor_x;
      let y = wa.cursor_y;
      if (x + width > rightEdge) x = rightEdge - width;
      if (y + height > bottomEdge) y = bottomEdge - height;
      if (x < wa.work_x) x = wa.work_x;
      if (y < wa.work_y) y = wa.work_y;

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
    border-radius: var(--radius-xl);
    border: 1px solid var(--border-strong);
    opacity: 0;
    transform: scale(0.92);
    transition:
      opacity var(--motion-default) ease-out,
      transform var(--motion-default) ease-out;
  }

  .preview-image.visible {
    opacity: 1;
    transform: scale(1);
  }
</style>
