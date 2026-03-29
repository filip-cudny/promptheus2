<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

  let src = $state("");
  const win = getCurrentWebviewWindow();

  async function loadImage() {
    const payload = await invoke<{
      data: string;
      media_type: string;
    } | null>("get_pending_image");
    if (payload) {
      src = `data:${payload.media_type};base64,${payload.data}`;
    }
  }

  function hide() {
    src = "";
    win.hide();
  }

  onMount(() => {
    const unlistenFocus = win.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        loadImage();
      } else {
        hide();
      }
    });

    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") hide();
    };
    window.addEventListener("keydown", handleKey);

    return () => {
      unlistenFocus.then((fn) => fn());
      window.removeEventListener("keydown", handleKey);
    };
  });
</script>

<div class="preview" role="button" tabindex="-1" onclick={hide} onkeydown={() => {}}>
  {#if src}
    <img {src} alt="Preview" class="preview-image" />
  {/if}
</div>

<style>
  .preview {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(30, 30, 30, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 8px;
    cursor: pointer;
  }

  .preview-image {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 7px;
  }
</style>
