<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

  let src = $state("");
  const win = getCurrentWebviewWindow();

  function hide() {
    win.hide();
  }

  onMount(() => {
    const unlisten = win.listen<{ data: string; media_type: string }>(
      "image-data",
      (event) => {
        src = `data:${event.payload.media_type};base64,${event.payload.data}`;
      },
    );

    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") hide();
    };
    window.addEventListener("keydown", handleKey);
    window.addEventListener("blur", hide);

    return () => {
      unlisten.then((fn) => fn());
      window.removeEventListener("keydown", handleKey);
      window.removeEventListener("blur", hide);
    };
  });
</script>

<div class="preview">
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
  }

  .preview-image {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 7px;
  }
</style>
