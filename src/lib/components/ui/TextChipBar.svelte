<script lang="ts">
  import AttachmentChip from "./AttachmentChip.svelte";

  const PREVIEW_MAX_CHARS = 200;

  type Variant = "default" | "small";

  let {
    textAttachments = $bindable(),
    readonly = false,
    variant = "default" as Variant,
    onremove,
    onopen,
  }: {
    textAttachments: string[];
    readonly?: boolean;
    variant?: Variant;
    onremove?: (index: number) => void;
    onopen?: (text: string, index: number) => void;
  } = $props();

  function removeAttachment(index: number) {
    if (onremove) {
      onremove(index);
    } else {
      textAttachments = textAttachments.filter((_, i) => i !== index);
    }
  }

  function truncate(text: string): string {
    if (text.length <= PREVIEW_MAX_CHARS) return text;
    return text.slice(0, PREVIEW_MAX_CHARS) + "…";
  }
</script>

{#each textAttachments as text, idx}
  <AttachmentChip label="Text #{idx + 1}" {readonly} {variant} onclick={() => onopen?.(text, idx)} onremove={() => removeAttachment(idx)}>
    {#snippet content()}
      <span class="chip-preview">{truncate(text)}</span>
    {/snippet}
  </AttachmentChip>
{/each}

<style>
  .chip-preview {
    font-size: 9px;
    line-height: var(--line-height-tight);
    color: var(--text-muted);
    overflow: hidden;
    word-break: break-word;
    padding: var(--space-2) var(--space-2) var(--space-0);
    height: 100%;
  }
</style>
