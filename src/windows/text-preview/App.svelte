<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import TextPreviewView from "$lib/components/features/text-preview/TextPreviewView.svelte";
  import { useTextPreviewLoad } from "$lib/components/features/text-preview/drivers/useTextPreviewLoad.svelte";

  const win = getCurrentWebviewWindow();
  const loader = useTextPreviewLoad();

  let text = $state("");
  let editMode = $state(false);
  let isDirty = $derived(text !== loader.originalText);

  function hide() {
    loader.commitAndHide(win.label, text);
  }

  async function copyText() {
    await navigator.clipboard.writeText(text);
  }

  onMount(() => {
    void loader.load().then((loaded) => {
      if (loaded === null) return;
      text = loaded;
      editMode = false;
    });

    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") hide();
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  });
</script>

<TextPreviewView bind:text bind:editMode {isDirty} onSave={hide} onCopy={copyText} />
