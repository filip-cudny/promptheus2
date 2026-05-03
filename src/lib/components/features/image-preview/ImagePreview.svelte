<script lang="ts">
  let {
    src,
    visible,
    onHide,
  }: {
    src: string;
    visible: boolean;
    onHide: () => void;
  } = $props();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="preview" onclick={onHide} onkeydown={() => {}}>
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
